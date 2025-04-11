use raw_cpuid::{CpuId, ProcessorBrandString};
use spin::Mutex;
use lazy_static::lazy_static;

/// CPU information
#[derive(Debug, Clone, Default)]
pub struct CpuInfo {
    pub vendor_id: String,
    pub brand_string: String,
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
    pub features: CpuFeatures,
}

#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    pub sse: bool,
    pub sse2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub hypervisor: bool,
}

lazy_static! {
    static ref CPU_INFO: Mutex<Option<CpuInfo>> = Mutex::new(None);
}

pub fn initialize() {
    let mut cpu_info_guard = CPU_INFO.lock();
    
    if cpu_info_guard.is_none() {
        let cpuid = CpuId::new();
        
        // Get vendor ID
        let vendor_id = cpuid
            .get_vendor_info()
            .map(|vendor| vendor.as_str().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
            
        // Get processor brand string
        let brand_string = cpuid
            .get_processor_brand_string()
            .map(|brand| brand.as_str().to_string())
            .unwrap_or_else(|| "Unknown Processor".to_string());
            
        // Get feature information
        let features = if let Some(feature_info) = cpuid.get_feature_info() {
            CpuFeatures {
                sse: feature_info.has_sse(),
                sse2: feature_info.has_sse2(),
                hypervisor: feature_info.has_hypervisor(),
                // More features can be added as needed
                ..Default::default()
            }
        } else {
            CpuFeatures::default()
        };
        
        let family_id = cpuid
            .get_feature_info()
            .map(|info| info.family_id())
            .unwrap_or(0);
            
        let model_id = cpuid
            .get_feature_info()
            .map(|info| info.model_id())
            .unwrap_or(0);
            
        let stepping = cpuid
            .get_feature_info()
            .map(|info| info.stepping_id())
            .unwrap_or(0);
        
        *cpu_info_guard = Some(CpuInfo {
            vendor_id,
            brand_string,
            family: family_id as u8,
            model: model_id as u8,
            stepping: stepping as u8,
            features,
        });
    }
}

pub fn get_cpu_info() -> Option<CpuInfo> {
    CPU_INFO.lock().clone()
}

pub fn detect_cpu() -> CpuInfo {
    initialize();
    CPU_INFO.lock().clone().unwrap_or_default()
}