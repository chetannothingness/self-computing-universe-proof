import KernelVm

open KernelVm

def zfc_zero_ne_oneProg : Program :=
  { instructions := [
      Instruction.push (0),
      Instruction.push (1),
      Instruction.eq,
      Instruction.jz 5,
      Instruction.halt 0,
      Instruction.halt 1
    ] }
