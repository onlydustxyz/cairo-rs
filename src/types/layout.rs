use std::prelude::v1::*;

use super::instance_definitions::{
    builtins_instance_def::BuiltinsInstanceDef, cpu_instance_def::CpuInstanceDef,
    diluted_pool_instance_def::DilutedPoolInstanceDef,
};

#[derive(Debug)]
pub(crate) struct CairoLayout {
    pub(crate) _name: String,
    pub(crate) _cpu_component_step: u32,
    pub(crate) rc_units: u32,
    pub(crate) builtins: BuiltinsInstanceDef,
    pub(crate) _public_memory_fraction: u32,
    pub(crate) _memory_units_per_step: u32,
    pub(crate) diluted_pool_instance_def: Option<DilutedPoolInstanceDef>,
    pub(crate) _n_trace_colums: u32,
    pub(crate) _cpu_instance_def: CpuInstanceDef,
}

impl CairoLayout {
    pub(crate) fn plain_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("plain"),
            _cpu_component_step: 1,
            rc_units: 16,
            builtins: BuiltinsInstanceDef::plain(),
            _public_memory_fraction: 4,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: None,
            _n_trace_colums: 8,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }

    pub(crate) fn small_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("small"),
            _cpu_component_step: 1,
            rc_units: 16,
            builtins: BuiltinsInstanceDef::small(),
            _public_memory_fraction: 4,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: None,
            _n_trace_colums: 25,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }

    pub(crate) fn dex_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("dex"),
            _cpu_component_step: 1,
            rc_units: 4,
            builtins: BuiltinsInstanceDef::dex(),
            _public_memory_fraction: 4,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: None,
            _n_trace_colums: 22,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }

    pub(crate) fn perpetual_with_bitwise_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("perpetual_with_bitwise"),
            _cpu_component_step: 1,
            rc_units: 4,
            builtins: BuiltinsInstanceDef::perpetual_with_bitwise(),
            _public_memory_fraction: 4,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: Some(DilutedPoolInstanceDef::new(2, 4, 16)),
            _n_trace_colums: 10,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }

    pub(crate) fn bitwise_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("bitwise"),
            _cpu_component_step: 1,
            rc_units: 4,
            builtins: BuiltinsInstanceDef::bitwise(),
            _public_memory_fraction: 8,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: Some(DilutedPoolInstanceDef::default()),
            _n_trace_colums: 10,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }

    pub(crate) fn recursive_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("recursive"),
            _cpu_component_step: 1,
            rc_units: 4,
            builtins: BuiltinsInstanceDef::recursive(),
            _public_memory_fraction: 8,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: Some(DilutedPoolInstanceDef::default()),
            _n_trace_colums: 11,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }

    pub(crate) fn all_instance() -> CairoLayout {
        CairoLayout {
            _name: String::from("all"),
            _cpu_component_step: 1,
            rc_units: 8,
            builtins: BuiltinsInstanceDef::all(),
            _public_memory_fraction: 8,
            _memory_units_per_step: 8,
            diluted_pool_instance_def: Some(DilutedPoolInstanceDef::default()),
            _n_trace_colums: 27,
            _cpu_instance_def: CpuInstanceDef::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_plain_instance() {
        let layout = CairoLayout::plain_instance();
        let builtins = BuiltinsInstanceDef::plain();
        assert_eq!(&layout._name, "plain");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 16);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 4);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(layout.diluted_pool_instance_def, None);
        assert_eq!(layout._n_trace_colums, 8);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }

    #[test]
    fn get_small_instance() {
        let layout = CairoLayout::small_instance();
        let builtins = BuiltinsInstanceDef::small();
        assert_eq!(&layout._name, "small");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 16);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 4);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(layout.diluted_pool_instance_def, None);
        assert_eq!(layout._n_trace_colums, 25);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }

    #[test]
    fn get_dex_instance() {
        let layout = CairoLayout::dex_instance();
        let builtins = BuiltinsInstanceDef::dex();
        assert_eq!(&layout._name, "dex");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 4);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 4);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(layout.diluted_pool_instance_def, None);
        assert_eq!(layout._n_trace_colums, 22);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }

    #[test]
    fn get_perpetual_with_bitwise_instance() {
        let layout = CairoLayout::perpetual_with_bitwise_instance();
        let builtins = BuiltinsInstanceDef::perpetual_with_bitwise();
        assert_eq!(&layout._name, "perpetual_with_bitwise");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 4);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 4);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(
            layout.diluted_pool_instance_def,
            Some(DilutedPoolInstanceDef::new(2, 4, 16))
        );
        assert_eq!(layout._n_trace_colums, 10);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }

    #[test]
    fn get_bitwise_instance() {
        let layout = CairoLayout::bitwise_instance();
        let builtins = BuiltinsInstanceDef::bitwise();
        assert_eq!(&layout._name, "bitwise");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 4);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 8);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(
            layout.diluted_pool_instance_def,
            Some(DilutedPoolInstanceDef::default())
        );
        assert_eq!(layout._n_trace_colums, 10);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }

    #[test]
    fn get_recursive_instance() {
        let layout = CairoLayout::recursive_instance();
        let builtins = BuiltinsInstanceDef::recursive();
        assert_eq!(&layout._name, "recursive");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 4);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 8);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(
            layout.diluted_pool_instance_def,
            Some(DilutedPoolInstanceDef::default())
        );
        assert_eq!(layout._n_trace_colums, 11);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }

    #[test]
    fn get_all_instance() {
        let layout = CairoLayout::all_instance();
        let builtins = BuiltinsInstanceDef::all();
        assert_eq!(&layout._name, "all");
        assert_eq!(layout._cpu_component_step, 1);
        assert_eq!(layout.rc_units, 8);
        assert_eq!(layout.builtins, builtins);
        assert_eq!(layout._public_memory_fraction, 8);
        assert_eq!(layout._memory_units_per_step, 8);
        assert_eq!(
            layout.diluted_pool_instance_def,
            Some(DilutedPoolInstanceDef::default())
        );
        assert_eq!(layout._n_trace_colums, 27);
        assert_eq!(layout._cpu_instance_def, CpuInstanceDef::default());
    }
}
