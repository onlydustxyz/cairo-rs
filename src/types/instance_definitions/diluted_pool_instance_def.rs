#[derive(Debug, PartialEq)]
pub(crate) struct DilutedPoolInstanceDef {
    pub(crate) units_per_step: u32,
    pub(crate) spacing: u32,
    pub(crate) n_bits: u32,
}

impl DilutedPoolInstanceDef {
    pub(crate) fn default() -> Self {
        DilutedPoolInstanceDef {
            units_per_step: 16,
            spacing: 4,
            n_bits: 16,
        }
    }

    pub(crate) fn new(units_per_step: u32, spacing: u32, n_bits: u32) -> Self {
        DilutedPoolInstanceDef {
            units_per_step,
            spacing,
            n_bits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DilutedPoolInstanceDef;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let diluted_pool = DilutedPoolInstanceDef::default();
        assert_eq!(diluted_pool.units_per_step, 16);
        assert_eq!(diluted_pool.spacing, 4);
        assert_eq!(diluted_pool.n_bits, 16);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let diluted_pool = DilutedPoolInstanceDef::new(1, 1, 1);
        assert_eq!(diluted_pool.units_per_step, 1);
        assert_eq!(diluted_pool.spacing, 1);
        assert_eq!(diluted_pool.n_bits, 1);
    }
}
