use {
    tracing::info,
    tch::{
        nn::{self, VarStore, ConvConfig, Conv2D, Linear, OptimizerConfig}, 
        Device,
    },
};

pub struct SimpleMnistModel {
    vs: VarStore,

    conv1: Conv2D,
    linear1: Linear,
}

impl SimpleMnistModel {
    pub fn new() -> Self {
        let vs = VarStore::new(Device::cuda_if_available());
        let root = vs.root();

        let conv1 = nn::conv2d(&root, 1, 3, 3, ConvConfig {
            padding: 1,
            ..Default::default()
        });

        let linear1 = nn::linear(&root, 28, 10, Default::default());

        Self {
            vs,

            conv1,
            linear1,
        }
    }

    pub fn run(&self) {
        // TODO
    }

    pub fn train(&self) {
        let mut npz = npyz::npz::NpzArchive::open("./mnist.npz").unwrap();
        let mut opt = nn::Adam::default().build(&self.vs, 1e-4).unwrap();
   
        for epoch in 0..100 {
            info!("running epoch {}", epoch);

            let mut iter = Iter2::new(&x_train, &y_train, 1024);
        }
    }
}