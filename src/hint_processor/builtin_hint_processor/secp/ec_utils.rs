use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::{
            hint_utils::{
                get_integer_from_var_name, get_relocatable_from_var_name, insert_value_into_ap,
            },
            secp::secp_utils::{pack, pack_from_relocatable, SECP_REM},
        },
        hint_processor_definition::HintReference,
    },
    math_utils::{ec_double_slope, line_slope},
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};
use std::{
    collections::HashMap,
    ops::{BitAnd, Shl},
};

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

    y = pack(ids.point.y, PRIME) % SECP_P
    # The modulo operation in python always returns a nonnegative number.
    value = (-y) % SECP_P
%}
*/
pub fn ec_negate(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = num_bigint::BigInt::one().shl(256u32)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .clone()
            .to_bigint();

    //ids.point
    let point_y = get_relocatable_from_var_name("point", vm, ids_data, ap_tracking)? + 3i32;
    let y = pack_from_relocatable(point_y, vm)?;
    let value = (-y).mod_floor(&secp_p);
    exec_scopes.insert_value("value", value);
    Ok(())
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
    from starkware.python.math_utils import ec_double_slope

    # Compute the slope.
    x = pack(ids.point.x, PRIME)
    y = pack(ids.point.y, PRIME)
    value = slope = ec_double_slope(point=(x, y), alpha=0, p=SECP_P)
%}
*/
pub fn compute_doubling_slope(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = num_bigint::BigInt::one().shl(256usize)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    //ids.point
    let point_reloc = get_relocatable_from_var_name("point", vm, ids_data, ap_tracking)?;

    let (x_d0, x_d1, x_d2, y_d0, y_d1, y_d2) = (
        vm.get_integer(&point_reloc)?,
        vm.get_integer(&(&point_reloc + 1i32))?,
        vm.get_integer(&(&point_reloc + 2i32))?,
        vm.get_integer(&(&point_reloc + 3i32))?,
        vm.get_integer(&(&point_reloc + 4i32))?,
        vm.get_integer(&(&point_reloc + 5i32))?,
    );

    let value = ec_double_slope(
        &(
            pack(x_d0.as_ref(), x_d1.as_ref(), x_d2.as_ref()),
            pack(y_d0.as_ref(), y_d1.as_ref(), y_d2.as_ref()),
        ),
        &BigInt::zero(),
        &secp_p,
    );
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("slope", value);
    Ok(())
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
    from starkware.python.math_utils import line_slope

    # Compute the slope.
    x0 = pack(ids.point0.x, PRIME)
    y0 = pack(ids.point0.y, PRIME)
    x1 = pack(ids.point1.x, PRIME)
    y1 = pack(ids.point1.y, PRIME)
    value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)
%}
*/
pub fn compute_slope(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = BigInt::one().shl(256usize)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    //ids.point0
    let point0_reloc = get_relocatable_from_var_name("point0", vm, ids_data, ap_tracking)?;

    let (point0_x_d0, point0_x_d1, point0_x_d2, point0_y_d0, point0_y_d1, point0_y_d2) = (
        vm.get_integer(&point0_reloc)?,
        vm.get_integer(&(&point0_reloc + 1i32))?,
        vm.get_integer(&(&point0_reloc + 2i32))?,
        vm.get_integer(&(&point0_reloc + 3i32))?,
        vm.get_integer(&(&point0_reloc + 4i32))?,
        vm.get_integer(&(&point0_reloc + 5i32))?,
    );

    //ids.point1
    let point1_reloc = get_relocatable_from_var_name("point1", vm, ids_data, ap_tracking)?;

    let (point1_x_d0, point1_x_d1, point1_x_d2, point1_y_d0, point1_y_d1, point1_y_d2) = (
        vm.get_integer(&point1_reloc)?,
        vm.get_integer(&(&point1_reloc + 1i32))?,
        vm.get_integer(&(&point1_reloc + 2i32))?,
        vm.get_integer(&(&point1_reloc + 3i32))?,
        vm.get_integer(&(&point1_reloc + 4i32))?,
        vm.get_integer(&(&point1_reloc + 5i32))?,
    );

    let value = line_slope(
        &(
            pack(
                point0_x_d0.as_ref(),
                point0_x_d1.as_ref(),
                point0_x_d2.as_ref(),
            ),
            pack(
                point0_y_d0.as_ref(),
                point0_y_d1.as_ref(),
                point0_y_d2.as_ref(),
            ),
        ),
        &(
            pack(
                point1_x_d0.as_ref(),
                point1_x_d1.as_ref(),
                point1_x_d2.as_ref(),
            ),
            pack(
                point1_y_d0.as_ref(),
                point1_y_d1.as_ref(),
                point1_y_d2.as_ref(),
            ),
        ),
        &secp_p,
    );
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("slope", value);
    Ok(())
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

    slope = pack(ids.slope, PRIME)
    x = pack(ids.point.x, PRIME)
    y = pack(ids.point.y, PRIME)

    value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P
%}
*/
pub fn ec_double_assign_new_x(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = BigInt::one().shl(256usize)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    //ids.slope
    let slope_reloc = get_relocatable_from_var_name("slope", vm, ids_data, ap_tracking)?;

    let (slope_d0, slope_d1, slope_d2) = (
        vm.get_integer(&slope_reloc)?,
        vm.get_integer(&(&slope_reloc + 1_i32))?,
        vm.get_integer(&(&slope_reloc + 2_i32))?,
    );

    //ids.point
    let point_reloc = get_relocatable_from_var_name("point", vm, ids_data, ap_tracking)?;

    let (x_d0, x_d1, x_d2, y_d0, y_d1, y_d2) = (
        vm.get_integer(&point_reloc)?,
        vm.get_integer(&(&point_reloc + 1i32))?,
        vm.get_integer(&(&point_reloc + 2i32))?,
        vm.get_integer(&(&point_reloc + 3i32))?,
        vm.get_integer(&(&point_reloc + 4i32))?,
        vm.get_integer(&(&point_reloc + 5i32))?,
    );

    let slope = pack(slope_d0.as_ref(), slope_d1.as_ref(), slope_d2.as_ref());
    let x = pack(x_d0.as_ref(), x_d1.as_ref(), x_d2.as_ref());
    let y = pack(y_d0.as_ref(), y_d1.as_ref(), y_d2.as_ref());

    let value = (slope.pow(2) - (&x << 1u32)).mod_floor(&secp_p);

    //Assign variables to vm scope
    exec_scopes.insert_value("slope", slope);
    exec_scopes.insert_value("x", x);
    exec_scopes.insert_value("y", y);
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("new_x", value);
    Ok(())
}

/*
Implements hint:
%{ value = new_y = (slope * (x - new_x) - y) % SECP_P %}
*/
pub fn ec_double_assign_new_y(
    exec_scopes: &mut ExecutionScopes,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = BigInt::one().shl(256usize)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    //Get variables from vm scope
    let (slope, x, new_x, y) = (
        exec_scopes.get::<BigInt>("slope")?,
        exec_scopes.get::<BigInt>("x")?,
        exec_scopes.get::<BigInt>("new_x")?,
        exec_scopes.get::<BigInt>("y")?,
    );

    let value = (slope * (x - new_x) - y).mod_floor(&secp_p);
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("new_y", value);
    Ok(())
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

    slope = pack(ids.slope, PRIME)
    x0 = pack(ids.point0.x, PRIME)
    x1 = pack(ids.point1.x, PRIME)
    y0 = pack(ids.point0.y, PRIME)

    value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P
%}
*/
pub fn fast_ec_add_assign_new_x(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = BigInt::one().shl(256usize)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    //ids.slope
    let slope_reloc = get_relocatable_from_var_name("slope", vm, ids_data, ap_tracking)?;

    let (slope_d0, slope_d1, slope_d2) = (
        vm.get_integer(&slope_reloc)?,
        vm.get_integer(&(&slope_reloc + 1i32))?,
        vm.get_integer(&(&slope_reloc + 2i32))?,
    );

    //ids.point0
    let point0_reloc = get_relocatable_from_var_name("point0", vm, ids_data, ap_tracking)?;

    let (point0_x_d0, point0_x_d1, point0_x_d2, point0_y_d0, point0_y_d1, point0_y_d2) = (
        vm.get_integer(&point0_reloc)?,
        vm.get_integer(&(&point0_reloc + 1i32))?,
        vm.get_integer(&(&point0_reloc + 2i32))?,
        vm.get_integer(&(&point0_reloc + 3i32))?,
        vm.get_integer(&(&point0_reloc + 4i32))?,
        vm.get_integer(&(&point0_reloc + 5i32))?,
    );

    //ids.point1.x
    let point1_reloc = get_relocatable_from_var_name("point1", vm, ids_data, ap_tracking)?;

    let (point1_x_d0, point1_x_d1, point1_x_d2) = (
        vm.get_integer(&point1_reloc)?,
        vm.get_integer(&(&point1_reloc + 1i32))?,
        vm.get_integer(&(&point1_reloc + 2i32))?,
    );

    let slope = pack(slope_d0.as_ref(), slope_d1.as_ref(), slope_d2.as_ref());
    let x0 = pack(
        point0_x_d0.as_ref(),
        point0_x_d1.as_ref(),
        point0_x_d2.as_ref(),
    );
    let x1 = pack(
        point1_x_d0.as_ref(),
        point1_x_d1.as_ref(),
        point1_x_d2.as_ref(),
    );
    let y0 = pack(
        point0_y_d0.as_ref(),
        point0_y_d1.as_ref(),
        point0_y_d2.as_ref(),
    );

    let value = (&slope * &slope - &x0 - &x1).mod_floor(&secp_p);
    //Assign variables to vm scope
    exec_scopes.insert_value("slope", slope);
    exec_scopes.insert_value("x0", x0);
    exec_scopes.insert_value("y0", y0);
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("new_x", value);

    Ok(())
}

/*
Implements hint:
%{ value = new_y = (slope * (x0 - new_x) - y0) % SECP_P %}
*/
pub fn fast_ec_add_assign_new_y(
    exec_scopes: &mut ExecutionScopes,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let secp_p = BigInt::one().shl(256usize)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    //Get variables from vm scope
    let (slope, x0, new_x, y0) = (
        exec_scopes.get::<BigInt>("slope")?,
        exec_scopes.get::<BigInt>("x0")?,
        exec_scopes.get::<BigInt>("new_x")?,
        exec_scopes.get::<BigInt>("y0")?,
    );
    let value = (slope * (x0 - new_x) - y0).mod_floor(&secp_p);
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("new_y", value);

    Ok(())
}

/*
Implements hint:
%{ memory[ap] = (ids.scalar % PRIME) % 2 %}
*/
pub fn ec_mul_inner(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    //(ids.scalar % PRIME) % 2
    let scalar = get_integer_from_var_name("scalar", vm, ids_data, ap_tracking)?
        .as_ref()
        .bitand(&Felt::one());
    insert_value_into_ap(vm, scalar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::builtin_hint_processor_definition::{
                BuiltinHintProcessor, HintProcessorData,
            },
            hint_processor_definition::HintProcessor,
        },
        types::{
            exec_scope::ExecutionScopes,
            relocatable::{MaybeRelocatable, Relocatable},
        },
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError, runners::builtin_runner::RangeCheckBuiltinRunner,
            vm_core::VirtualMachine, vm_memory::memory::Memory,
        },
    };
    use std::any::Any;

    #[test]
    fn run_ec_negate_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\n\ny = pack(ids.point.y, PRIME) % SECP_P\n# The modulo operation in python always returns a nonnegative number.\nvalue = (-y) % SECP_P";
        let mut vm = vm_with_range_check!();

        vm.memory = memory![((1, 3), 2645i32), ((1, 4), 454i32), ((1, 5), 206i32)];
        //Initialize fp
        vm.run_context.fp = 1;
        //Create hint_data
        let ids_data = ids_data!["point"];
        let mut exec_scopes = ExecutionScopes::new();
        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );
        //Check 'value' is defined in the vm scope
        assert_eq!(
            exec_scopes.get::<BigInt>("value"),
            Ok(bigint_str!(
                "115792089237316195423569751828682367333329274433232027476421668138471189901786"
            ))
        );
    }

    #[test]
    fn run_compute_doubling_slope_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\nfrom starkware.python.math_utils import ec_double_slope\n\n# Compute the slope.\nx = pack(ids.point.x, PRIME)\ny = pack(ids.point.y, PRIME)\nvalue = slope = ec_double_slope(point=(x, y), alpha=0, p=SECP_P)";
        let mut vm = vm_with_range_check!();
        vm.memory = memory![
            ((1, 0), 614323u64),
            ((1, 1), 5456867u64),
            ((1, 2), 101208u64),
            ((1, 3), 773712524u64),
            ((1, 4), 77371252u64),
            ((1, 5), 5298795u64)
        ];

        //Initialize fp
        vm.run_context.fp = 1;

        let ids_data = ids_data!["point"];
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );
        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "40442433062102151071094722250325492738932110061897694430475034100717288403728"
        )
                ),
                (
                    "slope",
                    bigint_str!(
            "40442433062102151071094722250325492738932110061897694430475034100717288403728"
        )
                )
            ]
        );
    }

    #[test]
    fn run_compute_slope_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\nfrom starkware.python.math_utils import line_slope\n\n# Compute the slope.\nx0 = pack(ids.point0.x, PRIME)\ny0 = pack(ids.point0.y, PRIME)\nx1 = pack(ids.point1.x, PRIME)\ny1 = pack(ids.point1.y, PRIME)\nvalue = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)";
        let mut vm = vm_with_range_check!();

        //Insert ids.point0 and ids.point1 into memory
        vm.memory = memory![
            ((1, 0), 134),
            ((1, 1), 5123),
            ((1, 2), 140),
            ((1, 3), 1232),
            ((1, 4), 4652),
            ((1, 5), 720),
            ((1, 6), 156),
            ((1, 7), 6545),
            ((1, 8), 100010),
            ((1, 9), 1123),
            ((1, 10), 1325),
            ((1, 11), 910)
        ];

        //Initialize fp
        vm.run_context.fp = 14;
        let ids_data = HashMap::from([
            ("point0".to_string(), HintReference::new_simple(-14)),
            ("point1".to_string(), HintReference::new_simple(-8)),
        ]);
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );
        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "41419765295989780131385135514529906223027172305400087935755859001910844026631"
        )
                ),
                (
                    "slope",
                    bigint_str!(
            "41419765295989780131385135514529906223027172305400087935755859001910844026631"
        )
                )
            ]
        );
    }

    #[test]
    fn run_ec_double_assign_new_x_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\n\nslope = pack(ids.slope, PRIME)\nx = pack(ids.point.x, PRIME)\ny = pack(ids.point.y, PRIME)\n\nvalue = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P";
        let mut vm = vm_with_range_check!();

        //Insert ids.point and ids.slope into memory
        vm.memory = memory![
            ((1, 0), 134),
            ((1, 1), 5123),
            ((1, 2), 140),
            ((1, 3), 1232),
            ((1, 4), 4652),
            ((1, 5), 720),
            ((1, 6), 44186171158942157784255469_i128),
            ((1, 7), 54173758974262696047492534_i128),
            ((1, 8), 8106299688661572814170174_i128)
        ];

        //Initialize fp
        vm.run_context.fp = 10;
        let ids_data = HashMap::from([
            ("point".to_string(), HintReference::new_simple(-10)),
            ("slope".to_string(), HintReference::new_simple(-4)),
        ]);
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "slope",
                    bigint_str!(
            "48526828616392201132917323266456307435009781900148206102108934970258721901549"
        )
                ),
                (
                    "x",
                    bigint_str!("838083498911032969414721426845751663479194726707495046")
                ),
                (
                    "y",
                    bigint_str!("4310143708685312414132851373791311001152018708061750480")
                ),
                (
                    "value",
                    bigint_str!(
            "59479631769792988345961122678598249997181612138456851058217178025444564264149"
        )
                ),
                (
                    "new_x",
                    bigint_str!(
            "59479631769792988345961122678598249997181612138456851058217178025444564264149"
        )
                )
            ]
        );
    }

    #[test]
    fn run_ec_double_assign_new_y_ok() {
        let hint_code = "value = new_y = (slope * (x - new_x) - y) % SECP_P";
        let mut vm = vm_with_range_check!();
        let mut exec_scopes = scope![
            (
                "slope",
                bigint_str!(
                    "48526828616392201132917323266456307435009781900148206102108934970258721901549"
                )
            ),
            (
                "x",
                bigint_str!("838083498911032969414721426845751663479194726707495046")
            ),
            (
                "new_x",
                bigint_str!(
                    "59479631769792988345961122678598249997181612138456851058217178025444564264149"
                )
            ),
            (
                "y",
                bigint_str!("4310143708685312414132851373791311001152018708061750480")
            )
        ];
        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                HashMap::new(),
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                ),
                (
                    "new_y",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                )
            ]
        );
    }

    #[test]
    fn run_fast_ec_add_assign_new_x_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\n\nslope = pack(ids.slope, PRIME)\nx0 = pack(ids.point0.x, PRIME)\nx1 = pack(ids.point1.x, PRIME)\ny0 = pack(ids.point0.y, PRIME)\n\nvalue = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P";
        let mut vm = vm_with_range_check!();

        //Insert ids.point0, ids.point1.x and ids.slope into memory
        vm.memory = memory![
            //ids.point0
            ((1, 0), 89712),
            ((1, 1), 56),
            ((1, 2), 1233409),
            ((1, 3), 980126),
            ((1, 4), 10),
            ((1, 5), 8793),
            //ids.point0.x
            ((1, 6), 1235216451),
            ((1, 7), 5967),
            ((1, 8), 2171381),
            //ids.slope
            ((1, 9), 67470097831679799377177424_i128),
            ((1, 10), 43370026683122492246392730_i128),
            ((1, 11), 16032182557092050689870202_i128)
        ];

        //Initialize run_context
        run_context!(vm, 0, 20, 15);

        let ids_data = HashMap::from([
            ("point0".to_string(), HintReference::new_simple(-15)),
            ("point1".to_string(), HintReference::new_simple(-9)),
            ("slope".to_string(), HintReference::new_simple(-6)),
        ]);
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "8891838197222656627233627110766426698842623939023296165598688719819499152657"
        )
                ),
                (
                    "new_x",
                    bigint_str!(
            "8891838197222656627233627110766426698842623939023296165598688719819499152657"
        )
                )
            ]
        );
    }

    #[test]
    fn run_fast_ec_add_assign_new_y_ok() {
        let hint_code = "value = new_y = (slope * (x0 - new_x) - y0) % SECP_P";
        let mut vm = vm_with_range_check!();

        let mut exec_scopes = scope![
            (
                "slope",
                bigint_str!(
                    "48526828616392201132917323266456307435009781900148206102108934970258721901549"
                )
            ),
            (
                "x0",
                bigint_str!("838083498911032969414721426845751663479194726707495046")
            ),
            (
                "new_x",
                bigint_str!(
                    "59479631769792988345961122678598249997181612138456851058217178025444564264149"
                )
            ),
            (
                "y0",
                bigint_str!("4310143708685312414132851373791311001152018708061750480")
            )
        ];

        //Execute the hint
        assert_eq!(
            run_hint!(
                vm,
                HashMap::new(),
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt::one().shl(32_u32)
                        + Felt::one().shl(9_u32)
                        + Felt::one().shl(8_u32)
                        + Felt::one().shl(7_u32)
                        + Felt::one().shl(6_u32)
                        + Felt::one().shl(4_u32)
                        + Felt::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                ),
                (
                    "new_y",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                )
            ]
        );
    }

    #[test]
    fn run_ec_mul_inner_ok() {
        let hint_code = "memory[ap] = (ids.scalar % PRIME) % 2";
        let mut vm = vm_with_range_check!();

        let scalar = 89712_i32;
        //Insert ids.scalar into memory
        vm.memory = memory![((1, 0), scalar)];

        //Initialize RunContext
        run_context!(vm, 0, 2, 1);

        let ids_data = ids_data!["scalar"];

        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));

        //Check hint memory inserts
        check_memory![&vm.memory, ((1, 2), 0)];
    }
}
