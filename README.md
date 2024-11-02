# OS project 2 code

This repo holds the code I modified for our first driver project. The original
redox project is broken up into many submodules, so it's likely not all of the
OS source code copied over to this repo.

The code relevant to this project is:

* [`cookbook/recipes/core/drivers/source/homework/src/main.rs`](./cookbook/recipes/core/drivers/source/homework/src/main.rs):
  This is the main function, i.e. the runtime, of the homework driver. It is a
  standalone program that runs in user space. It starts by registering itself
  as scheme provider with the OS. It then reads packets from an `event` scheme,
  handles them, and sends packets back to the caller via the `event` scheme.
* [`cookbook/recipes/core/drivers/source/homework/src/scheme.rs`](cookbook/recipes/core/drivers/source/homework/src/scheme.rs):
  This is the scheme functionality. Callers interact with the driver as if it
  was a file, so the driver interface consists of standardized file system
  calls. The scheme, however, is free to handle each call however it would
  like, within the confines of the interface (plus the confines of the standard
  library functions that call them).
* [`cookbook/recipes/test-homework/source/src/main.rs`](cookbook/recipes/test-homework/source/src/main.rs):
  This is the test program for the homework driver. It uses the driver path of
  `/scheme/homework` and interacts with the driver as a file. As I write this,
  I realize there's some cruft left in this file. For example, the buffer being
  initialized to `[0, 1, 0, 1]` was to see if I could pass info from the caller
  to the driver through this buffer. I cannot, as something (I think the std
  lib) clears the contents of the buffer before passing it to the driver
  (probably more secure that way).
