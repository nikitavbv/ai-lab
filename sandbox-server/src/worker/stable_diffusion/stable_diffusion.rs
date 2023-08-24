// copied from: https://github.com/huggingface/candle/blob/main/candle-examples/examples/stable-diffusion/stable_diffusion.rs

use super::schedulers::PredictionType;
use super::{clip, ddim, unet_2d, vae};
use candle::{DType, Device, Result};
use candle_nn as nn;

#[derive(Clone, Debug)]
pub struct StableDiffusionConfig {
    pub width: usize,
    pub height: usize,
    pub clip: clip::Config,
    autoencoder: vae::AutoEncoderKLConfig,
    unet: unet_2d::UNet2DConditionModelConfig,
    scheduler: ddim::DDIMSchedulerConfig,
}

impl StableDiffusionConfig {
    pub fn v1_5(
        sliced_attention_size: Option<usize>,
        height: Option<usize>,
        width: Option<usize>,
    ) -> Self {
        let bc = |out_channels, use_cross_attn, attention_head_dim| unet_2d::BlockConfig {
            out_channels,
            use_cross_attn,
            attention_head_dim,
        };
        // https://huggingface.co/runwayml/stable-diffusion-v1-5/blob/main/unet/config.json
        let unet = unet_2d::UNet2DConditionModelConfig {
            blocks: vec![
                bc(320, true, 8),
                bc(640, true, 8),
                bc(1280, true, 8),
                bc(1280, false, 8),
            ],
            center_input_sample: false,
            cross_attention_dim: 768,
            downsample_padding: 1,
            flip_sin_to_cos: true,
            freq_shift: 0.,
            layers_per_block: 2,
            mid_block_scale_factor: 1.,
            norm_eps: 1e-5,
            norm_num_groups: 32,
            sliced_attention_size,
            use_linear_projection: false,
        };
        let autoencoder = vae::AutoEncoderKLConfig {
            block_out_channels: vec![128, 256, 512, 512],
            layers_per_block: 2,
            latent_channels: 4,
            norm_num_groups: 32,
        };
        let height = if let Some(height) = height {
            assert_eq!(height % 8, 0, "heigh has to be divisible by 8");
            height
        } else {
            512
        };

        let width = if let Some(width) = width {
            assert_eq!(width % 8, 0, "width has to be divisible by 8");
            width
        } else {
            512
        };

        Self {
            width,
            height,
            clip: clip::Config::v1_5(),
            autoencoder,
            scheduler: Default::default(),
            unet,
        }
    }

    fn v2_1_(
        sliced_attention_size: Option<usize>,
        height: Option<usize>,
        width: Option<usize>,
        prediction_type: PredictionType,
    ) -> Self {
        let bc = |out_channels, use_cross_attn, attention_head_dim| unet_2d::BlockConfig {
            out_channels,
            use_cross_attn,
            attention_head_dim,
        };
        // https://huggingface.co/stabilityai/stable-diffusion-2-1/blob/main/unet/config.json
        let unet = unet_2d::UNet2DConditionModelConfig {
            blocks: vec![
                bc(320, true, 5),
                bc(640, true, 10),
                bc(1280, true, 20),
                bc(1280, false, 20),
            ],
            center_input_sample: false,
            cross_attention_dim: 1024,
            downsample_padding: 1,
            flip_sin_to_cos: true,
            freq_shift: 0.,
            layers_per_block: 2,
            mid_block_scale_factor: 1.,
            norm_eps: 1e-5,
            norm_num_groups: 32,
            sliced_attention_size,
            use_linear_projection: true,
        };
        // https://huggingface.co/stabilityai/stable-diffusion-2-1/blob/main/vae/config.json
        let autoencoder = vae::AutoEncoderKLConfig {
            block_out_channels: vec![128, 256, 512, 512],
            layers_per_block: 2,
            latent_channels: 4,
            norm_num_groups: 32,
        };
        let scheduler = ddim::DDIMSchedulerConfig {
            prediction_type,
            ..Default::default()
        };

        let height = if let Some(height) = height {
            assert_eq!(height % 8, 0, "heigh has to be divisible by 8");
            height
        } else {
            768
        };

        let width = if let Some(width) = width {
            assert_eq!(width % 8, 0, "width has to be divisible by 8");
            width
        } else {
            768
        };

        Self {
            width,
            height,
            clip: clip::Config::v2_1(),
            autoencoder,
            scheduler,
            unet,
        }
    }

    pub fn v2_1(
        sliced_attention_size: Option<usize>,
        height: Option<usize>,
        width: Option<usize>,
    ) -> Self {
        // https://huggingface.co/stabilityai/stable-diffusion-2-1/blob/main/scheduler/scheduler_config.json
        Self::v2_1_(
            sliced_attention_size,
            height,
            width,
            PredictionType::VPrediction,
        )
    }

    pub fn v2_1_inpaint(
        sliced_attention_size: Option<usize>,
        height: Option<usize>,
        width: Option<usize>,
    ) -> Self {
        // https://huggingface.co/stabilityai/stable-diffusion-2-inpainting/blob/main/scheduler/scheduler_config.json
        // This uses a PNDM scheduler rather than DDIM but the biggest difference is the prediction
        // type being "epsilon" by default and not "v_prediction".
        Self::v2_1_(
            sliced_attention_size,
            height,
            width,
            PredictionType::Epsilon,
        )
    }

    pub fn build_vae(&self, vae_weights: &str, device: &Device) -> Result<vae::AutoEncoderKL> {
        let weights = unsafe { candle::safetensors::MmapedFile::new(vae_weights)? };
        let weights = weights.deserialize()?;
        let vs_ae = nn::VarBuilder::from_safetensors(vec![weights], DType::F32, device);
        // https://huggingface.co/runwayml/stable-diffusion-v1-5/blob/main/vae/config.json
        let autoencoder = vae::AutoEncoderKL::new(vs_ae, 3, 3, self.autoencoder.clone())?;
        Ok(autoencoder)
    }

    pub fn build_unet(
        &self,
        unet_weights: &str,
        device: &Device,
        in_channels: usize,
    ) -> Result<unet_2d::UNet2DConditionModel> {
        let weights = unsafe { candle::safetensors::MmapedFile::new(unet_weights)? };
        let weights = weights.deserialize()?;
        let vs_unet = nn::VarBuilder::from_safetensors(vec![weights], DType::F32, device);
        let unet = unet_2d::UNet2DConditionModel::new(vs_unet, in_channels, 4, false, self.unet.clone())?;
        Ok(unet)
    }

    pub fn build_scheduler(&self, n_steps: usize) -> Result<ddim::DDIMScheduler> {
        ddim::DDIMScheduler::new(n_steps, self.scheduler)
    }

    pub fn build_clip_transformer(
        &self,
        clip_weights: &str,
        device: &Device,
    ) -> Result<clip::ClipTextTransformer> {
        let weights = unsafe { candle::safetensors::MmapedFile::new(clip_weights)? };
        let weights = weights.deserialize()?;
        let vs = nn::VarBuilder::from_safetensors(vec![weights], DType::F32, device);
        let text_model = clip::ClipTextTransformer::new(vs, &self.clip)?;
        Ok(text_model)
    }
}