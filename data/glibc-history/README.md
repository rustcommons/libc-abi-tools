# glibc ABI history evidence

The user reference is [GLIBC-SYMBOLS.md](../../docs/reference/GLIBC-SYMBOLS.md).
Scope, semantics, evidence gaps, and the conformance plan are described in
[GLIBC-HISTORY.md](../../docs/GLIBC-HISTORY.md).

`symbols.tsv` and `changes.tsv` derive from the 28 pinned upstream ABI snapshots.
Their separate [COPYING.abilists](../glibc/COPYING.abilists) license and
[upstream attribution](../../NOTICE.md) remain applicable; the project MIT license
does not relicense the ABI datasets or derived reference tables.

`elf-evidence.tsv` preserves public numeric GLIBC observations from independently
readelf-checked real input trees, labelled by release, target, input profile,
library/SONAME, symbol/version, and input/metadata hashes. Profile source archives,
build configuration, package manifests, and patches are recorded in `PROVENANCE.json`.
Default/native evidence is incomplete outside those profiles and must not be guessed.
No glibc implementation or runtime binaries are included here.

The early tag/archive catalogs are inventory candidates only, including development
tags and patches. Unknown hashes, historical interfaces, and target applicability
are explicit gaps. Refreshing this inventory is a deliberate source-collection task,
not an automatic network step in reference regeneration.

After independently qualifying a new real input tree against readelf and the
pinned snapshot, import it deliberately with `scripts/import-glibc-elf-profile.py`.
The importer checks input and metadata hashes and complete public membership,
preserves existing profiles, and refuses a duplicate profile name. Regenerate
the interface reference afterward. The GNU 2.18 profile was added this way;
native evidence now covers four profiles and 11673 public numeric observations.

Build `libc-abi`, then run `scripts/build-glibc-reference.py --binary PATH` to
regenerate the matrices and Markdown. Use `--check` to verify pinned outputs.
