use {
    std::{time::Duration, sync::Arc},
    tracing::info,
    tokio::{time::sleep, sync::Mutex},
    tonic::{
        service::Interceptor,
        metadata::MetadataValue,
        Status,
        Request,
    },
    sandbox_common::utils::{init_logging, load_config},
    rpc::{
        self,
        sandbox_service_client::SandboxServiceClient,
        GetTaskToRunRequest,
        UpdateTaskStatusRequest,
    },
    crate::{
        model::{StableDiffusionImageGenerationModel, ImageGenerationStatus},
        storage::Storage,
    },
};

pub mod model;
pub mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    let config = load_config();
    
    info!("sandbox worker started");
    let endpoint = config.get_string("worker.endpoint").unwrap();
    let client = Arc::new(Mutex::new(SandboxServiceClient::with_interceptor(
        tonic::transport::Channel::from_shared(endpoint)
            .unwrap()
            .connect()
            .await
            .unwrap(),
        AuthTokenSetterInterceptor::new(config.get_string("token.worker_token").unwrap()),
    )));
    
    let storage = Storage::new(&config);

    info!("loading model");
    let model = StableDiffusionImageGenerationModel::new(&storage).await;
    info!("model loaded");

    loop {
        let res = client.lock().await.get_task_to_run(GetTaskToRunRequest {}).await.unwrap().into_inner();
    
        let task = match res.task_to_run {
            Some(v) => v,
            None => {
                info!("no tasks at this moment, waiting...");
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        let prompt = task.prompt;
        let id = task.id.unwrap();
    
        info!("generating image for prompt: {}, task id: {}", prompt, id.id);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        {
            let id = id.clone();
            let client = client.clone();

            tokio::spawn(async move {
                while let Some(update) = rx.recv().await {
                    match update {
                        ImageGenerationStatus::Finished => break,
                        ImageGenerationStatus::InProgress { current_step, total_steps } => {
                            client.lock().await.update_task_status(UpdateTaskStatusRequest {
                                id: Some(id.clone()),
                                task_status: Some(rpc::update_task_status_request::TaskStatus::InProgress(rpc::InProgressTaskDetails {
                                    current_step,
                                    total_steps,
                                })),
                            }).await.unwrap();
                        },
                    }
                }
            });
        }
        
        let image = model.run(&prompt, tx);
        info!("finished generating image");
        
        client.lock().await.update_task_status(UpdateTaskStatusRequest {
            id: Some(id.clone()),
            task_status: Some(rpc::update_task_status_request::TaskStatus::Finished(rpc::FinishedTaskDetails {
                image,
            })),
        }).await.unwrap();

        info!("finished processing task");
    }
}

pub struct AuthTokenSetterInterceptor {
    token: String,
}

impl AuthTokenSetterInterceptor {
    pub fn new(token: String) -> Self {
        Self {
            token,
        }
    }
}

impl Interceptor for AuthTokenSetterInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        let auth_header_value: MetadataValue<tonic::metadata::Ascii> = MetadataValue::try_from(&self.token).expect("failed to create metadata");
        req.metadata_mut().insert("x-access-token", auth_header_value);
        Ok(req)
    }
}

