Update 001 — Programming on the metal — Late Mate
---

Hi! How is it going? Nikita here.

Welcome to our very first progress update!

No preorders yet, but we were working hard and thus feel it’s important to end the year with something tangible.

# Programming on the metal

I programmed all my life at the very top of programming languages pyramid, mindlessly utilizing hundreds of abstractions I didn’t even know existed. That’s why writing a program for microcontrollers fascinates me.

We chose RP2040 as our microcontroller, and this is what we have at our disposal:

- 133 MHz ARM Cortex-M0+ processor
- Two cores
- 264 Kb of RAM
- 2Mb of flash memory

## Cross-compilation

Adventure starts immediately: with cross-compilation. Raspberry Pi Pico runs a processor with a different architecture than my (and probably your) computer. This is not a technical challenge (in the end, the compiler consumes text and outputs bytes, and bytes are the same everywhere), but more of an organizational one (meaning we, programmers, made a mess out of it).

Luckily, Rust compiler knows how to deal with our particular case. Not having an OS or libc to depend on makes it easier, too.

## No allocations

Now, the limited memory. In most computers, you allocate memory left and right without thinking about it. There’s usually lots of it, and your program will probably finish long before it reaches the limit.
In our case, we only have 264 Kb and we have to work forever. Dynamic allocation has another very nasty problem: fragmentation. You might free 5 bytes here, 17 there, but when you need 20, you might not be able to find a contiguous segment of that size. It gets worse the longer your program runs, too.

To address that, we run without dynamic allocations at all (meaning you can’t use Rust’s standard library, so `no_std`). All memory is known at compile time and is statically allocated. No new memory can be claimed. Want to concatenate two strings? Declare a byte array of a reasonable size ahead of time.

## Debug output

The simplest, most commonly used form of debugging is println. But on the microcontroller, there’s nowhere to print. There’s no screen, no terminal, no pipes even — these are all OS abstractions, are we don’t have OS.

A microcontroller has pins, and it can apply some voltage to them, which can be interpreted as a sequence of zeroes and ones. Which is, to be fair, pretty low-level.

Meet [defmt](https://defmt.ferrous-systems.com/). It deconstructs all your printlns into two parts: a static list of string data and dynamic objects you splice into them. Objects are serialized and communicated through debug protocol to debug probe, and strings are all known at compile-time. The final console output is reconstructed on your (much more powerful) computer, saving the microcontroller precious cycles.

## No OS

Yes, a microcontroller runs no OS. There are no files, no sockets, no stdout, nothing. It executes the code you give it directly. So whatever you want it to do, you have to bring it yourself. Even a panic handler is [a library](https://docs.rs/panic-probe/0.3.1/panic_probe/). Or [atomics](https://github.com/rust-embedded/critical-section).

## Async

Async is usually considered a pretty high-level feature, and even some high-level languages might add it late in their lifespan.

Imagine my surprise finding out that async just works on this very primitive device we have with no help from anything except the library and the Rust language. Well done, Rust!

## Type-safe hardware

Considering Rust's goal of making everything safe, in the case of microcontrollers, they managed to make even hardware access safe! Get the right set of types for your board (in our case, [embassy-rp](https://github.com/embassy-rs/embassy/tree/main/embassy-rp)) and you can turn your head off and let the compiler catch you accessing hardware the wrong way.

## In conclusion

Programming for hardware in Rust feels weird: on one side, you have to think at the lowest possible level of detail, on the other, you get a relatively high-level language to work in.

But the overall experience is great, and we even get to (potentially) share code between the microcontroller, our CLI, and our GUI app.

Well done, Rust!

# Competition tease

We watched [CS2's Input Latency](https://www.youtube.com/watch?v=NE0qg_8k0BE) with great interest. Apparently, nVidia is making a very similar device called “LDAT” and sells it to selected reviewers and developers for $1500.

Compared to them, we plan to attack the problem from a different angle:

- Better industrial design (sorry, no finicky elastic straps)
- Statistically significant amounts of measurements (hundreds or - thousands, completely automated)
- Better feedback in the companion app
- Much more affordable price tag
- Available to everyone

# Board progress

Dan has asked me to share this picture:

Not sure what to make of it but no doubt it’s important.

That’s about it. Happy Christmas, and see you all in 2024!