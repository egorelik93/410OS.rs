# 410OS.rs

This project is an attempt at porting my CMU 15-410 kernel, which I wrote with a partner in college in C, to Rust. In a bit of irony, Rust 1.0 came out in the final few weeks of that semester.

The initial goal is to be a faithful port of the C implementation as of the state it was submitted in.
That includes various known bugs and design flaws! Some issues have been addressed where Rust
has caught them - rather than attempt to temporarily fight Rust on these, I am already "Rustifying" 
such code. Further fixes and design improvements will wait until after the initial port is complete.
At that time, I may proceed with experimenting with how much an advanced type system can benefit kernel
development.

Out of respect for the course, the supplied course code and ports thereof are not part of this repo. That does mean that the repo will not build on its own.





