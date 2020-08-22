vulkan_samples_2019_rust
====

[3DグラフィクスAPI Vulkanを出来るだけやさしく解説する本](https://fadis.booth.pm/items/1562222) のサンプルコードの Rust での実装を試みました。

https://github.com/Fadis/vulkan_samples_2019

[assimp](https://www.assimp.org/) の Rust binding の導入がうまくいかなかったので下記は未実装 (または途中まで) です。

* 00_mesh
* 15_draw

Vulkan のライブラリとして [vulkano](https://github.com/vulkano-rs/vulkano) を利用していましたが、 [vk-mem](https://github.com/gwihlidal/vk-mem-rs) が使えないため途中で [ash](https://github.com/MaikKlein/ash) に切り替えています。

## 準備

vk-mem のビルドのため使用する toolchain に応じた C++ コンパイラーが必要です。

[GLFW](https://www.glfw.org/) のビルド済バイナリが必要です。 lib フォルダーに使用する toolchain に応じたビルド済の lib, dll 等をコピーしてください。

