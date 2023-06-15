func main{}() {
  alloc_locals;
  local x;
  // This assertion will be skipped by vm.skip_next_instruction_execution
  %{
    skip_next_instruction()
   %}
  // This is an instruction of size 1
  [ap] = 4;
  [ap] = 5, ap++;

  // This is an instruction of size 2
  %{
    skip_next_instruction()
   %}
 call should_fail;

  x = 2;
  assert x = 2;
  return ();
}

func should_fail{}(a: felt) {
  assert a = 1512482385392052380;
  return ();
}
