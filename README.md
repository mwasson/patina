# patina
Patina is a small NES emulator that can currently play most mapper 0 games. That includes Super Mario Bros. and the classic "black box" games, as well as any
other games originally released before 1986. If you find a bug, please drop
me a line at mike.wasson+patina@gmail.com.

# Running

Run Patina at the command line. It expects a single argument, the path of
the ROM to play e.g.:

```
./patina foo.nes
```

# Controls

Controls are currently hardwired as so:

* B button: z
* A button: x
* D-Pad: directional arrows
* Select button: tab
* Start button: return

# Building

Assuming you have Rust and cargo installed, in the root directory, simply run:

```
cargo build --release

```

which will generate an executable, `target/release/patina`.

# Known Issues

Patina is not cycle accurate, so F-1 Race and Mach Rider do not yet render
correctly. The DMC is not fully implemented or tested; in particular, DMC IRQs
are not hooked up.

# Ethos and Project Goals 

Patina development began by Mike Wasson in March 2025. The original goals of 
the project were:

- to learn Rust,
- explore the limits of how an LLM could be used to assist in software development
- create an NES emulator that can run the original Super Mario Bros.

It turns out having an LLM write code isn't very interesting or fun for what's
essentially an educational project, so I wrote all the code myself and used
Claude only to clarify misunderstandings or explore alternate library choices.
Development picked up steam after July 2025, when I was laid off and had
more time for personal projects.

Some goals I'd like to eventually implement:
- Full sound
- Implement at least the popular mappers: MMC1, MMC3, etc.
- Save states
- Rewind
- Cycle-accurate behavior
- Test code suite
- Pass most of the popular test roms
- GUI menus, so users don't have to go into the command line
- Configurable controls, including for controller 2
- Efficiency: currently takes about twice as much CPU time as Nestopia
