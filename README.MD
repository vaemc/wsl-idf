# wsl-idf

## 配置esptool
`export ESPTOOL='D:\\esptool-windows-amd64\\esptool.exe'` esptool.exe必须放到windows系统上面

### 配置wsl-idf
`alias wsl-idf='$HOME/rust-project/wsl-idf/target/release/wsl-idf'`


## 用法
在esp-idf项目先运行idf.py build，编译项目成功后执行
* 烧录固件 wsl-idf -c flash -p COM1
* 合并固件  wsl-idf -c merge -p COM1
* 擦除后再烧录 wsl-idf -c flashx -p COM1
* 擦除 wsl-idf -c erase -p COM1