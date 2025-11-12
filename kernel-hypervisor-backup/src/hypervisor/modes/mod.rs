//! Hypervisor mode implementations
//!
//! This module contains the different virtualization mode implementations:
//! - type1: Full hardware virtualization
//! - virtio: Paravirtualization with VirtIO
//! - hvt: Solo5-style hardware virtualized tender

pub mod type1;
pub mod virtio;
pub mod hvt;

use super::{HypervisorError, Result};
use super::mode::{HypervisorMode, HypervisorModeImpl, ModeConfig};

/// Create a hypervisor mode implementation based on the mode type
pub fn create_mode(mode: HypervisorMode, config: &ModeConfig) -> Result<Box<dyn HypervisorModeImpl>> {
    match mode {
        HypervisorMode::Type1 => {
            let type1 = type1::Type1Hypervisor::init(config)?;
            Ok(Box::new(type1))
        }
        HypervisorMode::VirtIO => {
            let virtio = virtio::VirtIOHypervisor::init(config)?;
            Ok(Box::new(virtio))
        }
        HypervisorMode::Hvt => {
            let hvt = hvt::HvtTender::init(config)?;
            Ok(Box::new(hvt))
        }
    }
}
