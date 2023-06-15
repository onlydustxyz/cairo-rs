Custom Hint Processor
===

In order to customize the processing of hints in the Cairo Rust VM, users can create their own hint processor implementing the trait HintProcessor.

## How does the HintProcessor work?

In order to implement the trait, the processor must implement two methods: `compile_hint`, and `execute_hint`.
Note: The trait itself is defined in hint_processor_definition.rs

### `compile_hint`

This method is called before the execution phase of the VM.
It receives the Hint's data:

* The hint code as a String
* A map from variable name (note that this contains the full path of the variable, ie "__main__.a", instead of just the variable name "a") to reference id number
* A map of all the variable references (as a HintReference struct) by id (this id corresponds to the reference id number in the previous map)
* The hint's ap tracking data.

And it returns a dynamic structure, that will then be used by execute Hint.
The purpose of this method is to organize the data related to hints in the way it should be used by the processor to execute the hint.

### `execute_hint`

This method is called at the start of each VM step when there is a hint to execute.
It receives the dynamic structure created by `compile_hint` along with the program constants and a set of proxies containing a limited access to the VM's Internals:

* `exec_scopes_proxy` is the hint's gateway to interact with the execution_scopes in the VM and share data between hints without inserting them into the cairo execution. It provides methods to create and remove scopes and to modify the current scope, along with several helper methods to allow inserting and retrieving variables of specific types. This proxy only allows modifying the current scope, which is the last available scope before the hint's execution (Note that calling enter_scope and exit_scope won't change the current scope for the duration of the hint´s execution)
* `vm_proxy` is the hint's gateway to the internal values of the VM, it provides mutable references to the memory segment manager and the run context, and immutable references to the builtin runners and the program's prime, it also contains a memory proxy:
* `MemoryProxy`: It grants a more limited access to the VM's memory, providing the necessary methods to modify it in a controlled manner.

The purpose of this method is to carry out the execution of the hint, given the data from `compile_hint`

## Managing Cairo variables inside hint execution

Each variable's address and value can be computed with the information provided by the data in the HintReference structure + the hint's ap tracking data.
The following helper functions are provided in hint_processor_utils.rs to simplify variable management:

* get_integer_from_reference
* get_ptr_from_reference
* compute_addr_from_reference
* insert_value_from_reference

These methods take the HintReference associated to the variable along with the hint's ApTracking data.

Note: When handling pointer type variables, computing the address and using it to get the variable from memory might not lead to the correct value (as the variable reference may contain an immediate value that has to be added to the ptr itself), so using the function get_ptr_from_reference is strongly recommended.
Note: Cairo's memory is write-once, read-only, so when using `insert_value_from_reference` it's important to first make sure that the variable doesn't contain any value (for example, it may be defined as local but never written) to avoid inconsistent memory errors

## BuiltinHintProcessor

The BuiltinHintProcessor is the default hint executor of the VM, it is able to execute hints from the common library + sha256

## Usage Example

This is a simple example of a HintProcessor that can process the following hint:

```python
from starkware.cairo.common.math_utils import assert_integer
assert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128
assert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW
assert_integer(ids.value)
ids.low = ids.value & ((1 << 128) - 1)
ids.high = ids.value >> 128
```

We need to create our HintProcessor implementing the HintProcessor trait
```rust
pub struct MyHintProcessor {}

const SPLIT_FELT = : &str = r#"from starkware.cairo.common.math_utils import assert_integer
assert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128
assert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW
assert_integer(ids.value)
ids.low = ids.value & ((1 << 128) - 1)
ids.high = ids.value >> 128"#;

impl HintProcessor for MyHintProcessor {
    fn compile_hint(
        &self,
        code: String,
        ap_tracking: &ApTracking,
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
    ) -> Result<Box<dyn Any>, VirtualMachineError> {
        Ok(Box::new(HintProcessorData {
            code,
            ap_tracking: ap_tracking.clone(),
            ids_data: get_ids_data(reference_ids, references)?,
        }) as Box<dyn Any>)
    }

    fn execute_hint(
        &mut self,
        vm_proxy: &mut VMProxy,
        exec_scopes_proxy: &mut ExecutionScopesProxy,
        hint_data: &Box<dyn Any>,
    ) -> Result<(), VirtualMachineError> {
        let hint_data = hint_data
            .downcast_ref::<HintProcessorData>()
            .ok_or(VirtualMachineError::WrongHintData)?;
        match &*hint_data.code {
            SPLIT_FELT => split_felt(vm_proxy, &hint_data.ids_data, &hint_data.ap_tracking),
            _ => Err(VirtualMachineError::UnknownHint(code.to_string())),
        }
}
```

This is a helper function that organizes the data in the format that will be used by the executor
```rust
fn get_ids_data(
    reference_ids: &HashMap<String, usize>,
    references: &HashMap<usize, HintReference>,
) -> Result<HashMap<String, HintReference>, VirtualMachineError> {
    let mut ids_data = HashMap::<String, HintReference>::new();
    for (path, ref_id) in reference_ids {
        let name = path
            .rsplit('.')
            .next()
            .ok_or(VirtualMachineError::FailedToGetIds)?;
        ids_data.insert(
            name.to_string(),
            references
                .get(ref_id)
                .ok_or(VirtualMachineError::FailedToGetIds)?
                .clone(),
        );
    }
    Ok(ids_data)
}
```

This is the hint's implementation using the provided data and helpers:
```rust
pub fn split_felt(
    vm_proxy: &mut VMProxy,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), VirtualMachineError> {
    let value = get_integer_from_reference(vm_proxy, ids_data.get("value")?, ap_tracking)?;
    let low: BigInt = value & ((bigint!(1).shl(128_u8)) - bigint!(1));
    let high: BigInt = value.shr(128_u8);
    insert_value_from_reference(high, vm_proxy, ids_data.get("high")?, ap_tracking)?;
    insert_value_from_reference(low, vm_proxy, ids_data.get("low")?, ap_tracking)
}
```
