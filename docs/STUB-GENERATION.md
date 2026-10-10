# Experimental link only generation

The expanded emitter renders x86_64 assembly and GNU version scripts from
selected real ELF definitions. It preserves observed symbol versions/defaults,
binding, visibility, kinds, object/TLS sizes, and declared section alignment.
The selected-symbol experiment remains available; a separate
[full-library experiment](FULL-LIBRARIES.md) now covers the complete pinned public
numeric GLIBC 2.17 ABI. Neither is a released provider or stable artifact contract.

`stub::render` is separate from ABI collection and accepts parsed reference
metadata plus requested names. It emits every observed version of each name.
The glibc example additionally checks those definitions against the pinned 2.17
snapshot before writing any output. Reference SONAME and versions alone do not
authenticate a file or establish its source release.

## Emission rules

- Ordinary functions and IFUNC resolvers contain a two-byte trap. Their ELF types
  remain distinct. Function code sizes are deliberately replaced, not copied.
- Objects and TLS use zero-filled BSS/TBSS storage with recorded sizes. Initial
  values and actual TLS/runtime implementations are supplied by real libraries.
- Each emitted storage range is aligned to at least its original section's
  declared alignment. This is conservative placement, not a recovered C type
  alignment or a reconstruction of original data layout.
- Exact nonzero section/value/type/size matches share emitted storage. This
  retains an observed physical relationship without inventing semantic alias
  direction. Overlapping object/TLS views now use a bounded union with preserved
  offsets/sizes. Incompatible storage types and unsupported code overlaps fail.
- GNU assembler [symbol-version aliases](https://sourceware.org/binutils/docs/as/Symver.html)
  preserve both default and nondefault definitions. Version nodes sort numerically;
  original version-parent graphs are not reconstructed.
- Internal implementation names are localized by the
  [linker version script](https://sourceware.org/binutils/docs/ld/VERSION.html).
  Generated DSOs must retain the reference SONAME.

Supported bindings are global/weak, visibilities default/protected, and types
FUNC/IFUNC/OBJECT/TLS. GNU_UNIQUE, hidden/internal visibility, unversioned and
nonnumeric namespaces, special section indexes, zero-sized data/TLS, unsafe
assembler names, duplicate definitions/defaults, and unsupported overlaps are
rejected. Limits are 10000 requested names, 100000 reference definitions,
4096-byte alignment, 16 MiB per storage range, and a 64 MiB conservative allocation
budget. No assembler/linker is invoked by the Rust library.

## Reproduce the expanded experiment

With the existing two glibc input trees and Cargo on PATH:

```sh
bash examples/glibc-two-paths/expanded-and-check.sh /tmp/libc-abi-two-paths-clean
```

Requires x86_64 Linux, GCC, GNU binutils, and Python 3. The generated directories
`expanded-stub-prebuilt`, `expanded-stub-source`, and `expanded-fixture` must not
already exist. Failed writes can leave a partial directory for inspection.
Inputs and real runtime libraries remain in the isolated experiment directory.

The glibc example selects six names, producing seven definitions:
`__libc_start_main`, `puts`, both `memcpy` versions, `stdout`, `fwrite`, and `fflush`.
Each input route uses its own ELF evidence. The program explicitly calls
`memcpy@GLIBC_2.2.5` and the default `memcpy@GLIBC_2.14`, then writes through
`stdout`. The default memcpy export retains IFUNC type; stdout is an eight-byte
object and the executable exercises a copy relocation.

An independent original fixture selects multi-version functions, an IFUNC,
protected weak binding, sized data, and TLS. Its source explicitly defines the
tested function/data aliases. Executables linked with the generated fixture
instead run with the original real fixture and each real glibc 2.17 loader/libc.
They check both function versions, IFUNC results, TLS initialization/read/write,
data contents, and alias address identity. Relocation inspection confirms TLS
`R_X86_64_TPOFF64` and data `R_X86_64_COPY`.

On 2026-10-09 all four executables passed on the newer WSL kernel. Independent
readelf audits checked generated versions/defaults, types, binding, visibility,
sizes, alignment, SONAME, and shared ranges. Link maps retained real startup and
nonshared objects while supplying only generated shared libraries. Runtime
library lists selected only real libraries; no generated search path was embedded.
The newer memfd_create API still failed to link in both glibc routes.

Reports, source/generated hashes, and executable hashes are in
`logs/expanded-manifest.json` and `logs/expanded-*.txt`. This coverage does not
qualify complete libc libraries, general semantic aliases, multiple threads,
other releases, Rust integration, or an old distribution/SLES deployment.

The full-library follow-up additionally passes a Rust allocation/thread/TLS/unwind
program with both old runtimes. The next integration step is a provider artifact
contract and broader build/runtime coverage.
