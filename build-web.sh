RUSTFLAGS=--cfg=web_sys_unstable_apis trunk build --release --public-url simplez_asm template.html
mv dist/* .