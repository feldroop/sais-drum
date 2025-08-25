# 🥁 SAIS-drum 🥁

A Rust implementation of the Suffix Array Induced Sort (SAIS) algorithm for [suffix array](https://en.wikipedia.org/wiki/Suffix_array) construction. Inspired by Ilya Grebnov's [`libsais`](https://github.com/IlyaGrebnov/libsais) and based on the following paper:

> G. Nong, S. Zhang and W. H. Chan: _Two Efficient Algorithms for Linear Time Suffix Array Construction_ (2011) DOI: [10.1109/TC.2010.188](https://www.doi.org/10.1109/TC.2010.188)

## State of the implementation

The algorithm is implemented and tested using [`proptest`](https://github.com/proptest-rs/proptest), but not yet fully optimized. I highly recommend using my [bindings](https://github.com/feldroop/libsais-rs) to `libsais` instead. Other Rust solutions include Amos Wenger's port of [`divsufsort`](https://github.com/fasterthanlime/stringsearch/tree/master/crates/divsufsort) and Andrew Gallant's [`suffix`](https://github.com/BurntSushi/suffix) crate.

In the future, the following optimizations could be added (inspired by `libsais`):

- Algorithmic improvements laid out in this paper:

  > N. Timoshevskaya and W. -c. Feng: _SAIS-OPT: On the characterization and optimization of the SA-IS algorithm for suffix array construction_ (2014) DOI: [10.1109/ICCABS.2014.6863917](https://www.doi.org/10.1109/ICCABS.2014.6863917)

- Multithreading, based on this paper:

  > Lao, B., Nong, G., Chan, W.H. et al. : _Fast induced sorting suffixes on a multicore machine_ (2018) DOI: [10.1007/s11227-018-2395-5](https://doi.org/10.1007/s11227-018-2395-5)

- Implementation techniques laid out by Ilya Grebnov in the README of libsais
- General optimizations such as writing vectorization-friendly code
- Some of my own ideas that leverage Rust-specific features such as the easy creation of generic code compared to C

## Why drum?

Who doesn't like drums? Also, it's a pretty funny wordplay in German.
