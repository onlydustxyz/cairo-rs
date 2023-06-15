#[derive(Debug, PartialEq)]
pub(crate) struct CpuInstanceDef {
    pub(crate) _safe_call: bool,
}

impl CpuInstanceDef {
    pub(crate) fn default() -> Self {
        CpuInstanceDef { _safe_call: true }
    }
}

#[cfg(test)]
mod tests {
    use super::CpuInstanceDef;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let cpu_instance = CpuInstanceDef::default();
        assert!(cpu_instance._safe_call)
    }
}
