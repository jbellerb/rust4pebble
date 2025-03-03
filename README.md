# rust4pebble

<img align="right" width="160" height="350" src="media/watch.svg">

Safe and expressive Rust bindings for the [Pebble SDK](https://developer.rebble.io/developer.pebble.com/index.html). A project for [Rebble Hackathon #002](https://rebble.io/hackathon-002/).

> [!CAUTION]
> While I intend to write complete bindings, this is a massive undertaking and probably won't be done for a while. As of right now, expect nothing to work.

### Goals

During the hackathon, my goal is to finish at least the following:
- [ ] Get the SDK linking via Cargo
- [x] (Unsafe) low-level bindings to the SDK in [pebble-sys](pebble-sys/)
- [ ] A simple "Hello World!"-style app running within the emulator
- [ ] Documentation

Eventually:
- [ ] Higher-level safe library wrapping the SDK
- [ ] Scripts for publishing to the Rebble App Store
- [ ] Better documentation

<br />

#### License

<sup>
Copyright (C) jae beller, 2025.
</sup>
<br />
<sup>
Released under the <a href="https://www.gnu.org/licenses/lgpl-3.0.txt">GNU Lesser General Public License, Version 3</a> or later. See <a href="LICENSE">LICENSE</a> for more information.
</sup>
