# Attribution

PathsumWMC independently reimplements Huang et al., “Equivalence Checking of Quantum Circuits via Path-Sum and Weighted Model Counting” (TACAS 2026, arXiv:2604.24504), using exact arithmetic and rigorous ball bounds. The original reimplementation was prepared with Cursor, as stated in README.md.

2026-11-15: The balanced input/output circuit schedule is inspired by the two-sided strategies of QuPRS (PhysicsQoo/QuPRS, paper-era revision 60bcb250). Its implementation and the input-side gate transformations were written independently here and checked against an independent state-vector reference. No QuPRS verification source was copied into this implementation. The immutable preparation cache and exact dyadic GPMC numeric adapter are original PathsumWMC code.

GPMC and GANAK retain their upstream counting/search implementations and their original notices. Engine/dependency revisions, corresponding source archives and separate third-party licenses are recorded in web/public/engines/PROVENANCE.json and distributed with the browser build. PathsumWMC's MIT license does not replace third-party licenses.
