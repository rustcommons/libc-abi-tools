# Original MIT-licensed test data for controlled link/runtime checks.
.text
.globl old_foo
.type old_foo,@function
old_foo:
  mov $1,%eax
  ret
.size old_foo,.-old_foo
.symver old_foo,foo@GLIBC_2.2.5
.globl new_foo
.type new_foo,@function
new_foo:
  mov $2,%eax
  ret
.size new_foo,.-new_foo
.symver new_foo,foo@@GLIBC_2.14
.weak weak_function
.protected weak_function
.set weak_function,new_foo
.globl dispatch
.type dispatch,@gnu_indirect_function
dispatch:
  lea new_foo(%rip),%rax
  ret
.size dispatch,.-dispatch
.data
.balign 8
.globl shared_data
.type shared_data,@object
shared_data:
  .quad imported_symbol
.size shared_data,.-shared_data
.weak weak_data
.set weak_data,shared_data
.section .tdata,"awT",@progbits
.balign 4
.globl tls_data
.type tls_data,@tls_object
tls_data:
  .long 0
.size tls_data,.-tls_data
.section .note.GNU-stack,"",@progbits
