pub(crate) const CELLS_PER_RANGE_CHECK: u32 = 1;

#[derive(Debug, PartialEq)]
pub(crate) struct RangeCheckInstanceDef {
    pub(crate) ratio: Option<u32>,
    pub(crate) n_parts: u32,
}

impl RangeCheckInstanceDef {
    pub(crate) fn default() -> Self {
        RangeCheckInstanceDef {
            ratio: Some(8),
            n_parts: 8,
        }
    }

    pub(crate) fn new(ratio: Option<u32>, n_parts: u32) -> Self {
        RangeCheckInstanceDef { ratio, n_parts }
    }

    pub(crate) fn _cells_per_builtin(&self) -> u32 {
        CELLS_PER_RANGE_CHECK
    }

    pub(crate) fn _range_check_units_per_builtin(&self) -> u32 {
        self.n_parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_units_per_builtin() {
        let builtin_instance = RangeCheckInstanceDef::default();
        assert_eq!(builtin_instance._range_check_units_per_builtin(), 8);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_builtin() {
        let builtin_instance = RangeCheckInstanceDef::default();
        assert_eq!(builtin_instance._cells_per_builtin(), 1);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let builtin_instance = RangeCheckInstanceDef {
            ratio: Some(10),
            n_parts: 10,
        };
        assert_eq!(RangeCheckInstanceDef::new(Some(10), 10), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = RangeCheckInstanceDef {
            ratio: Some(8),
            n_parts: 8,
        };
        assert_eq!(RangeCheckInstanceDef::default(), builtin_instance);
    }
}
