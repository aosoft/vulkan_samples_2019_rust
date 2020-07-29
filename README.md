### 02_list_devices

* 列挙した Extension のバージョン表示がない
* device layers の列挙がない (vulkano が非サポート)
    * vulkano 内のコードにコメントあり (Device layers were deprecated in Vulkan 1.0.13)

### 07_create_descriptor_set

* vulkano では DescriptorSet が Pipeline に事実上隠蔽化されているので DescriptorSet 単体での初期化ができなさそう。
