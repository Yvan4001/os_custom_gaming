//! CPU features and capabilities

use raw_cpuid::{CpuId, ProcessorBrandString};

/// Required CPU features for OS Gaming
const REQUIRED_FEATURES: &[CpuFeature] = &[
    CpuFeature::SSE2,
    CpuFeature::AVX,
    CpuFeature::CMPXCHG8B,
];

/// CPU features that benefit gaming performance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFeature {
    SSE,
    SSE2,
    SSE3,
    SSSE3,
    SSE4_1,
    SSE4_2,
    AVX,
    AVX2,
    AVX512F,
    BMI1,
    BMI2,
    POPCNT,
    CMPXCHG8B,
    CMPXCHG16B,
    RDTSC,
    AES,
    PCLMULQDQ,
    XSAVE,
    OSXSAVE,
    F16C,
    FMA,
    MMX,
    FXSR,
    TSC,
    MSR,
}

/// Check if CPU has all required features
pub fn has_required_features() -> bool {
    REQUIRED_FEATURES.iter().all(|&feature| has_feature(feature))
}

/// Check if CPU has a specific feature
pub fn has_feature(feature: CpuFeature) -> bool {
    let cpuid = CpuId::new();
    
    // Check for feature based on CPUID
    match feature {
        CpuFeature::SSE => cpuid.get_feature_info()
            .map_or(false, |f| f.has_sse()),
        CpuFeature::SSE2 => cpuid.get_feature_info()
            .map_or(false, |f| f.has_sse2()),
        CpuFeature::SSE3 => cpuid.get_feature_info()
            .map_or(false, |f| f.has_sse3()),
        CpuFeature::SSSE3 => cpuid.get_feature_info()
            .map_or(false, |f| f.has_ssse3()),
        CpuFeature::SSE4_1 => cpuid.get_feature_info()
            .map_or(false, |f| f.has_sse41()),
        CpuFeature::SSE4_2 => cpuid.get_feature_info()
            .map_or(false, |f| f.has_sse42()),
        CpuFeature::AVX => cpuid.get_feature_info()
            .map_or(false, |f| f.has_avx()),
        CpuFeature::AVX2 => cpuid.get_extended_feature_info()
            .map_or(false, |f| f.has_avx2()),
        CpuFeature::POPCNT => cpuid.get_feature_info()
            .map_or(false, |f| f.has_popcnt()),
        CpuFeature::CMPXCHG8B => cpuid.get_feature_info()
            .map_or(false, |f| f.has_cmpxchg8b()),
        CpuFeature::CMPXCHG16B => cpuid.get_feature_info()
            .map_or(false, |f| f.has_cmpxchg16b()),
        _ => false, // Other features would need appropriate checks
    }
}

/// Optimize CPU caches for gaming workloads
pub fn optimize_cache_for_gaming() {
    // This would normally involve:
    // 1. Disabling prefetchers for certain workloads
    // 2. Setting appropriate cache allocation strategy
    // 3. Configuring memory type range registers (MTRRs)
    
    // These operations are highly specific to CPU models and require
    // privileged operations, so we're just stubbing the function here.
    // In a real implementation, this would use Model Specific Registers (MSRs)
    
    // Example (pseudocode):
    // - Disable hardware prefetchers
    // - Set cache write-back policy
    // - Configure MTRRs for frame buffer
}

/// Get the set of all supported features
pub fn get_supported_features() -> Vec<CpuFeature> {
    use CpuFeature::*;
    let all_features = [
        SSE, SSE2, SSE3, SSSE3, SSE4_1, SSE4_2, AVX, AVX2, AVX512F,
        BMI1, BMI2, POPCNT, CMPXCHG8B, CMPXCHG16B, RDTSC, AES,
        PCLMULQDQ, XSAVE, OSXSAVE, F16C, FMA, MMX, FXSR, TSC, MSR,
    ];
    
    all_features.iter()
        .filter(|&&feature| has_feature(feature))
        .cloned()
        .collect()
}