# RustyCraft

> **This project is learning project for rust and wgpu. Don't take it too serious**

## Introduction

This a basic minecraft clone and it has implemented:

-   Map generation via fbm noise
-   Placing and removing blocks
-   Chunk culling based on camera frustum
-   AABB and Raycasting for collision detection
-   Multiple render passes for translucency and ui
-   Save and load chunks/player states
-   Ambient occlusion and directional light

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
