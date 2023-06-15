# StarkNet cairo-rs hint design thoughts

## Requirements for hints
- Must cairo-rs support reuse of existing hints in starknet? Is it enough to enable re-writing these hints as python-embedded-in-cairo that uses a new API to interface with the cairo-rs VM state?
- Aside from mutable access to the memory manager and the registers, what do non-builtin hints require in terms of mutating the VM state? Is there anything that is explicitly not allowed or desirable, or that is currently allowed but would be better restricted?
- What is the medium-long term plan for hints? Be able to implement hints as rust-embedded-in-cairo, support python hints ad aeternum, add support for more languages?
- We should go through the implementation of the syscall hints and think how they can be implemented with the current cairo-rs code, what is missing, etc.

Required functionality that is currently not implemented by cairo-rs:
- Temporary Segments: 
  - segments.add_temp_segment()
  - (this segments is also relocated -> check for added relocation rules)
- Output Builtin -> add_page method
- vm_load_program()
- be able to run more than one hint runner at the same time
- Hints use constants -> we can add dictionary of constants to VMProxy

---

## Alternatives
0. Manually adding hint implementations (current way)

Notes
- depend on cairo-rs
- create helper crate with new hint implementations
- instantiate cairo vm in your rust program
- instantiate hint runner, set up with hints from the custom hint helper crate
- (-) hints must be implemented in rust, current hints unreusable
- (-) changes in starknet must be tracked and mirrored
- (+) good performance
- (+) already doable

1. embedded python runtime (write a hint runner which embeds a python interpreter)
    a. cpython crate ([example](https://github.com/dgrunwald/rust-cpython#example-program-displaying-the-value-of-sysversion))
    b. pyoxidizer + pyo3 ([some documentation](https://pyoxidizer.readthedocs.io/en/stable/pyoxidizer_overview.html#how-it-works))
    c. rustpython

Notes
- Create new crate, with a new hint runner, building on top of the builtinrunner but adding an embedded python interpreter
- How to provide the embedded python interpreter access to cairo-rs VM state?
- How hard will converting between type representations be? 
- One possibility that can be explored is writing a new implementation of MemorySegmentManager in python which works together with the python embedding mechanism, so that hints that get passed a reference to the MSM will be able to access cairo-rs instead of the python vm
- Another possibility that might be necessary is modifying the starknet hints in python to use a new interface mechanism.
- Of the three embedding options, cpython seems the most straightforward but also limited, pyo3 the most powerful, and rustpython the least mature
- From [here](https://www.infoworld.com/article/3664124/how-to-use-rust-with-python-and-python-with-rust.html):
      An important caveat with both cpython and PyO3 is to always minimize the number of times data is passed back and forth between the two languages.
      Each call from Python to Rust or vice versa incurs some overhead.
      If the overhead outweighs the work you're doing in Rust, you won't see any significant performance improvement.
      As an example, if you're looping over an object collection, send the object to Rust and perform the looping there.
      This is more efficient than looping on the Python side and calling the Rust code with each iteration of the loop.
      This guideline also applies generally to integrations between Python and other code that uses the Python C ABI, such as Cython modules.

2. Protocol + external process
    - Run python cairo & starknet code in a separate python process started by cairo-rs, with RPC style communication
	- How to provide access to vm state, and allow state modification by hints?
	- Should the hint code running in python intermittingly call an API to modify cairo-rs VM state, or should it run in completion, and send changes back to cairo-rs once done?
	- Should this protocol send and receive a full vm state representation, or just some representation of the changes to state made by the hint?
	- Open question: aside from the api to indirectly modify cairo-rs vm state, what else is needed to allow hints to run? dependencies?
	- How will having this python codebase affect testing, packaging, running, etc?
	- (+) more easily extensible to other languages?
	- (-) likely more overhead from state (de)serialization

3. Embedded webassembly runtime + hint compilation to wasm
    a. just hints compiled to wasm
    b. all cairo compiled to wasm

Notes
- Learn to use wasm tooling
- Learn how to compile python to wasm
- Definir the interfaces between the embedded wasm vm and cairo-rs
- (-) More unknowns & possibly greater resulting complexity
- (+) Greater flexibility, extensibility, and possibly performance

4. instead of embedding python un rust, do it the other way and embed cairo-rs in python cairo, replacing the current vm
     - Many questions, such doubt

---
## Starknet Hints

We went through the code in the starknet folder looking for hints, seeing what they use and depend on: 

### Modules list
* apps/amm_sample/amm_sample.cairo -> No hints
* common/constants.cairo -> No hints
* common/eth_utils.cairo -> No hints
* common/messages.cairo :heavy_check_mark: 
* common/storage.cairo :heavy_check_mark:
* common/syscalls.cairo :heavy_check_mark:
* core/os/block_context.cairo :heavy_check_mark:
* core/os/builtins.cairo :heavy_check_mark:
* core/os/contract_address/contract_address.cairo -> No hints
* core/os/contracts.cairo :heavy_check_mark:
* core/os/os.cairo :heavy_check_mark:
* core/os/os_config/os_config.cairo -> No hints
* core/os/output.cairo :heavy_check_mark:
* core/os/state.cairo :heavy_check_mark:
* core/os/transaction_hash/transaction_hash.cairo -> No hints
* core/os/transactions.cairo :heavy_check_mark:
* core/test_contract/delegate_proxy.cairo -> No hints
* core/test_contract/dummy_account.cairo -> No hints
* security/starknet_common.cairo -> No hints
* testing/test.cairo -> No hints
* testing/test_unwhitelisted_hint.cairo
* third_party/open_zeppelin/Account.cairo -> No hints
* third_party/open_zeppelin/utils/constants.cairo -> No hints

### common library
* **storage.cairo (1 hint)**: Only one hint that uses Cairo constants and variables just like the ones from the Cairo common library.
* **syscalls.cairo (14 hints)**: All hints appearing here use the object `syscall_handler` and its public API's, and all these take as arguments the VM memory segments and a pointer, `syscall_ptr`:
    * call_contract()
    * library_call()
    * library_call_l1_handler()
    * deploy()
    * get_caller_address()
    * get_sequencer_address()
    * get_block_number()
    * get_contract_address()
    * get_block_timestamp()
    * get_tx_signature()
    * storage_read()
    * storage_write()
    * emit_event()
    * get_tx_info()
* **messages.cairo (1 hint)**: Only one hint, using the `syscall_handler`:
    * send_message_to_l1()

### core/os library
* **contracts.cairo (4 hints)**: One of the hints uses a function `get_contract_class_struct` from the starknet module `starkware.starknet.core.os.class_hash` 
* **builtins.cairo**: There are no hints here but there is a dependency on builtins from the Cairo common library that we have not currently implemented:
    * EcOpBuiltin (it is implemented)
    * SignatureBuiltin
* **state.cairo (4 hints)**: Uses the patricia module from the Cairo common library. Hints use a object `global_state_storage` and methods associated to it.
* **block_context.cairo (5 hints)**: All hints are used with the `nondet` keyword. Also, the hints use the `sycall_handler` already mentioned and a new object, `os_input`.
* **transactions.cairo (46 hints)**: Has a dependency with the `builtin_selection` module of the Cairo library, a module with hints that we didn´t implement. Lots of syscalls in the hints.
* **output.cairo (1 hint)**: Just one big hint of regular Python code for the exception of the usage of the output_builtin as an object (I'm not sure if this appeared before)
* **os.cairo (3 hints)**: The module uses builtins ecdsa and ec_op, not currently implemented by our VM. One of the hints uses a class `StarknetOsInput` from the starknet module `starkware.starknet.core.os.os_input`.

