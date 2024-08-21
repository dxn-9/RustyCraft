# RustyCraft

> **This project is a learning project for rust and wgpu. Don't take it too serious**

## Introduction

This is a basic minecraft clone with the following features:

-   Map generation via fbm noise
-   Placing and removing blocks
-   Chunk culling based on camera frustum
-   AABB and Raycasting for collision detection
-   Multiple render passes for translucency and ui
-   Save and load chunks/player states
-   Ambient occlusion and directional light

---

_Commands:_

(WASD) for moving, (Scroll wheel / J-K) change placing block, (G) to toggle flying mode, (Space) jumping

## Building

Make sure you have rustc and cargo installed. Run the following command:

```bash
cargo run --release
```

![screenshot2](https://github.com/dandn9/RustyCraft/blob/media/house_screenshot.png)

![screenshot1](https://github.com/dandn9/RustyCraft/blob/media/world_screenshot.png)

## Configuration

Most of the configurations are done through constants in world.rs file.
