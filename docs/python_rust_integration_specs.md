LambdaClass - StarkWare
# Cairo-rs - Python FFI integration
#### 20th September 2022

### OVERVIEW
In order to allow cairo-rs to execute any Python code embedded into a Cairo program and to allow a Python context to call cairo-rs, we are adding support to provide communication of the VM state between Rust and Python via FFI bindings using PyO3, as an external crate. 

### GOAL
* Be able to efficiently and conveniently allow interactions between Python and cairo-rs. 
* Have an external crate which encapsulates the Python hint execution, and which is able to both run cairo-rs, and be imported as a Python module so that the VM can be ran from a Python process.

### SPECIFICATION
* FFI integration and functionality will be encapsulated in a crate external to cairo-rs. This crate will be called cairo-rs-py.
* The crate cairo-rs-py will behave as a cairo-rs VM wrapper, which can also be imported as a Python module. 
* The cairo-rs-py VM can be run for a set number of steps.
* The cairo-rs-py VM can be run for a set number of steps, paused, and then continue its execution.
* Variables defined by a hint can only be accessed by hints implemented in the same language, i.e., Rust hints are aware only of variables defined by Rust hints and Python hints are aware only of variables defined by Python hints. By Rust hints we refer to those implemented by the built-in hint processor.	 	
* The cairo-rs-py VM can be instantiated by a Python interpreter as a regular object. 
* A Rust or Python program can instantiate one or more independent cairo-rs-py VMs, allowing for multiple coexisting VMs.
* When instantiated by a Python interpreter, that same interpreter will be used to execute Python hints. 
* Python hints have limited access to the running context (code paths, modules, scopes created by previous hints).
* The syscall handler will be instantiated before the VM run and should be available on the hint locals.
* An instance of a cairo-rs-py VM will be running either a cairo program interpretation loop or a Python hint, but not both at the same time.
	* i.e. hints do not run concurrently 
	* The VM state shared with hints can only be accessed by a single hint at a time.
	* The VM memory is private to a VM instance and cannot be shared across different VM instances.
	* An instance of a VM will always run on the same thread.
	* Multiple instances of a VM can run on the same thread.
* Hint interaction with the VM will be restricted to:
	* read-write access to memory (with the methods __setitem__ and  __getitem__),
	* segments manager (with the methods add, write_arg, get_segments_used_sizes, add_temporary_segments),
	* ids (with the methods __setattr__ and __getattr__),
	* hint execution scopes,
	* read-only access to ap and fp registers.
	
* Nice to have: 
	* Drop the GIL during Rust operation to allow to parallelism when using multi-threads instead of process.
	* The cairo-rs-py VM can be instantiated by a Rust program, still allowing Python hints in Cairo programs.
	* Python hints are supported when running the cairo-rs-py standalone binary (as opposed to importing it from Python) only with the CPython interpreter.
