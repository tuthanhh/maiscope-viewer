
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]



<!-- PROJECT LOGO -->
<br />
<div align="center">
<h3 align="center">maiscope-viewer</h3>

  <p align="center">
    A rhythm game chart viewer for maimai, built with Rust and Bevy.
    <br />
    <a href="https://github.com/tuthanhh/maiscope-viewer/issues/new?labels=bug">Report Bug</a>
    &middot;
    <a href="https://github.com/tuthanhh/maiscope-viewer/issues/new?labels=enhancement">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#about-the-project">About The Project</a></li>
    <li><a href="#built-with">Built With</a></li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

**maiscope-viewer** is a desktop chart viewer for [maimai](https://maimai.sega.jp/) — an arcade rhythm game by SEGA. It parses the community **simai** chart format and renders all note types in real time, synchronized to the song's audio.

Built as a personal learning project to explore Bevy's ECS architecture and real-time 2D rendering in Rust.

**What it renders:**
- Tap, TapHold (bar hold)
- Touch, TouchHold (touchscreen sensor notes)
- Slide (star + path traces, including fan shapes)
- All note phases: approach → hit → after-effect burst

**How sync works:** the chart clock is anchored to the BGM's actual playback position every frame, so visual and audio never drift regardless of frame-time variance.




## Built With

[![Rust][Rust-badge]][Rust-url]
[![Bevy][Bevy-badge]][Bevy-url]

| Crate | Role |
|---|---|
| [`bevy`](https://bevyengine.org) 0.18 | ECS game engine, rendering, asset loading |
| [`bevy_kira_audio`](https://github.com/NiklasEi/bevy_kira_audio) | BGM + SFX playback via Kira |
| [`bevy_prototype_lyon`](https://github.com/Nilirad/bevy_prototype_lyon) | Lyon-backed 2D shape rendering |




<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable, 1.80+)
- A simai chart (`maidata.txt`) and its audio file — charts are community-created and typically distributed alongside song packages in the maimai fan community

### Installation

1. Clone the repo
   ```sh
   git clone https://github.com/tuthanhh/maiscope-viewer.git
   cd maiscope-viewer
   ```

2. Place your song assets under `assets/songs/<song_name>/`:
   ```
   assets/
   └── songs/
       └── YourSong/
           ├── maidata.txt   # simai chart file
           └── track.mp3     # audio file
   ```

3. Update the hardcoded song path in `src/systems/chart_playback.rs` and `src/systems/audio.rs` to point to your song folder.

4. Run
   ```sh
   cargo run
   ```
   For a faster build in release mode:
   ```sh
   cargo run --release
   ```




<!-- USAGE -->
## Usage

On launch the viewer loads the chart, waits for all assets, then starts playback automatically. Notes approach the judgment ring in sync with the BGM.

| Note type | Visual |
|---|---|
| Tap | Pink ring approaches outer ring |
| TapHold | Ring + extending hold bar |
| Touch | Blue diamond cluster converges to center |
| TouchHold | Blue hold with countdown arc |
| Slide | Star approaches ring, trace star follows path |

There is no interactive input — this is a viewer only. This is a personal project and is not open for contributions at this time.




<!-- ROADMAP -->
## Roadmap

- [ ] CLI song selection (currently hardcoded path)
- [ ] More playback option controls (note speed, chart speed, offset)
- [ ] More in-viewer information (judgement, scoring, BPM display)
- [ ] Broader simai format coverage (all edge cases)
- [ ] BPM-synced background visual

See [open issues](https://github.com/tuthanhh/maiscope-viewer/issues) for tracked items.




<!-- LICENSE -->
## License

Distributed under the MIT License.




<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

- [mai-notes.com](https://mai-notes.com/) — simai format reference and note timing documentation
- [majdata.net](https://majdata.net/) — additional chart format reference




<!-- MARKDOWN LINKS & IMAGES -->
[contributors-shield]: https://img.shields.io/github/contributors/tuthanhh/maiscope-viewer.svg?style=for-the-badge
[contributors-url]: https://github.com/tuthanhh/maiscope-viewer/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/tuthanhh/maiscope-viewer.svg?style=for-the-badge
[forks-url]: https://github.com/tuthanhh/maiscope-viewer/network/members
[stars-shield]: https://img.shields.io/github/stars/tuthanhh/maiscope-viewer.svg?style=for-the-badge
[stars-url]: https://github.com/tuthanhh/maiscope-viewer/stargazers
[issues-shield]: https://img.shields.io/github/issues/tuthanhh/maiscope-viewer.svg?style=for-the-badge
[issues-url]: https://github.com/tuthanhh/maiscope-viewer/issues
[license-shield]: https://img.shields.io/github/license/tuthanhh/maiscope-viewer.svg?style=for-the-badge
[license-url]: https://github.com/tuthanhh/maiscope-viewer/blob/master/LICENSE
[Rust-badge]: https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white
[Rust-url]: https://www.rust-lang.org/
[Bevy-badge]: https://img.shields.io/badge/Bevy-232326?style=for-the-badge&logo=bevy&logoColor=white
[Bevy-url]: https://bevyengine.org/
