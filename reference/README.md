# Pinned official sources

[source-lock.json](source-lock.json) records the official spaCy, Thinc, MurmurHash, and model files used as references. The exporter checks installed files against their wheel records. Keep this capture unchanged during ordinary setup and verification. Running `tools/provenance.py` replaces it; use that command only when reviewing a deliberate reference update, including changed files, licenses, fixtures, and installer compatibility.

## Verified wheel variation

spaCy 3.8.14's CPython 3.12 macOS and Linux wheels contain different build-directory comments in the generated `spacy/matcher/levenshtein.c` file. Comparing the complete files found no other changes. The Python, Cython, and other source files in the checked Linux spaCy, Thinc, and MurmurHash wheels matched the pinned capture.

The Linux artifact is `spacy-3.8.14-cp312-cp312-manylinux2014_x86_64.manylinux_2_17_x86_64.whl`, obtained from [the official PyPI release](https://pypi.org/project/spacy/3.8.14/#files). Its complete archive SHA-256 is `6d45715a24446f23b98ec3f09409a1d4111983d1d64613250ee38c3270e21853`.

| File variant | SHA-256 of `spacy/matcher/levenshtein.c` |
|---|---|
| Pinned macOS capture | `3d0aaaea19850900ec4071700ee6fe69c7d8e59ee1a4f9c963189e78161a278d` |
| Verified Linux wheel | `e14722922674055c3d72e21fd8b727597c58ba7395f70be68eaa55aa15bd44a9` |

`tools/installer_provenance.py` accepts this exact alternate hash for this file and version when regenerating the installer recipe. Unknown hashes, other source changes, changed versions, and changed licenses still fail. The exported provenance records the files observed on that machine; the immutable installer recipe retains its original capture. `tools/test_installer_provenance.py` tests both acceptance and rejection paths.
