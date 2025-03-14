# Dev commands

tailwinds
watchexec -w templates -r ~/Applications/tailwindcss --input public/css/base.css --output public/css/style.css -m

watch actix
watchexec -w templates -w src -r cargo run
