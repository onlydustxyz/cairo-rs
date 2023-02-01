use std::prelude::v1::*;

use super::{
    bitwise_instance_def::BitwiseInstanceDef, ec_op_instance_def::EcOpInstanceDef,
    ecdsa_instance_def::EcdsaInstanceDef, keccak_instance_def::KeccakInstanceDef,
    pedersen_instance_def::PedersenInstanceDef, range_check_instance_def::RangeCheckInstanceDef,
};

#[derive(Debug, PartialEq)]
pub(crate) struct BuiltinsInstanceDef {
    pub(crate) _output: bool,
    pub(crate) pedersen: Option<PedersenInstanceDef>,
    pub(crate) range_check: Option<RangeCheckInstanceDef>,
    pub(crate) _ecdsa: Option<EcdsaInstanceDef>,
    pub(crate) bitwise: Option<BitwiseInstanceDef>,
    pub(crate) ec_op: Option<EcOpInstanceDef>,
    pub(crate) keccak: Option<KeccakInstanceDef>,
}

impl BuiltinsInstanceDef {
    pub(crate) fn plain() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: false,
            pedersen: None,
            range_check: None,
            _ecdsa: None,
            bitwise: None,
            ec_op: None,
            keccak: None,
        }
    }

    pub(crate) fn small() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: true,
            pedersen: Some(PedersenInstanceDef::default()),
            range_check: Some(RangeCheckInstanceDef::default()),
            _ecdsa: Some(EcdsaInstanceDef::default()),
            bitwise: None,
            ec_op: None,
            keccak: None,
        }
    }

    pub(crate) fn dex() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: true,
            pedersen: Some(PedersenInstanceDef::default()),
            range_check: Some(RangeCheckInstanceDef::default()),
            _ecdsa: Some(EcdsaInstanceDef::default()),
            bitwise: None,
            ec_op: None,
            keccak: None,
        }
    }

    pub(crate) fn perpetual_with_bitwise() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: true,
            pedersen: Some(PedersenInstanceDef::new(32, 1)),
            range_check: Some(RangeCheckInstanceDef::new(16, 8)),
            _ecdsa: Some(EcdsaInstanceDef::new(2048)),
            bitwise: Some(BitwiseInstanceDef::new(64)),
            ec_op: Some(EcOpInstanceDef::new(1024)),
            keccak: None,
        }
    }

    pub(crate) fn bitwise() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: true,
            pedersen: Some(PedersenInstanceDef::new(256, 1)),
            range_check: Some(RangeCheckInstanceDef::default()),
            _ecdsa: Some(EcdsaInstanceDef::new(1024)),
            bitwise: Some(BitwiseInstanceDef::new(8)),
            ec_op: None,
            keccak: None,
        }
    }

    pub(crate) fn recursive() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: true,
            pedersen: Some(PedersenInstanceDef::new(256, 1)),
            range_check: Some(RangeCheckInstanceDef::default()),
            _ecdsa: None,
            bitwise: Some(BitwiseInstanceDef::new(16)),
            ec_op: None,
            keccak: Some(KeccakInstanceDef::new(2048, vec![200; 8])),
        }
    }

    pub(crate) fn all() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            _output: true,
            pedersen: Some(PedersenInstanceDef::default()),
            range_check: Some(RangeCheckInstanceDef::default()),
            _ecdsa: Some(EcdsaInstanceDef::default()),
            bitwise: Some(BitwiseInstanceDef::default()),
            ec_op: Some(EcOpInstanceDef::default()),
            keccak: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_builtins_plain() {
        let builtins = BuiltinsInstanceDef::plain();
        assert!(!builtins._output);
        assert!(builtins.pedersen.is_none());
        assert!(builtins.range_check.is_none());
        assert!(builtins._ecdsa.is_none());
        assert!(builtins.bitwise.is_none());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    fn get_builtins_small() {
        let builtins = BuiltinsInstanceDef::small();
        assert!(builtins._output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins._ecdsa.is_some());
        assert!(builtins.bitwise.is_none());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    fn get_builtins_dex() {
        let builtins = BuiltinsInstanceDef::dex();
        assert!(builtins._output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins._ecdsa.is_some());
        assert!(builtins.bitwise.is_none());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    fn get_builtins_perpetual_with_bitwise() {
        let builtins = BuiltinsInstanceDef::perpetual_with_bitwise();
        assert!(builtins._output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins._ecdsa.is_some());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_some());
    }

    #[test]
    fn get_builtins_bitwise() {
        let builtins = BuiltinsInstanceDef::bitwise();
        assert!(builtins._output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins._ecdsa.is_some());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    fn get_builtins_recursive() {
        let builtins = BuiltinsInstanceDef::recursive();
        assert!(builtins._output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins._ecdsa.is_none());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    fn get_builtins_all() {
        let builtins = BuiltinsInstanceDef::all();
        assert!(builtins._output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins._ecdsa.is_some());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_some());
    }
}
