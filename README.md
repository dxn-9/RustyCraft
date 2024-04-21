# RustyCraft

> **This project was used as learning project in order to learn rust and wgpu.**

## Introduction

This a basic minecraft clone and it includes features such as:

-   Heightmap generation via fbm noise map and trees
-   Placing and removing blocks
-   AO
-   Simple AABB and Raycasting for collision detection
-   Multiple render passes for different type of objects
-   Save and load modified chunks/player state
-   Chunk culling based of frustum

---

_Commands:_

(WASD) for moving , (Scroll wheel / J-K) change placing block, (G) to toggle flying mode, (Space) jumping

## Building

Make sure you have rustc and cargo installed an run the following command:

```bash
cargo run --release
```

![screenshot2](https://github.com/dandn9/RustyCraft/blob/media/house_screenshot.png)

![screenshot1](https://github.com/dandn9/RustyCraft/blob/media/world_screenshot.png)

## Configuration

Most of the configuration for the generation is done through constants in world.rs file.
