file_dir="../bin/file_directory"
cd "$(dirname "$0")"
mkdir -p $file_dir
printf %s "Hello, World! How are you doing today?" > $file_dir/hello.txt
curl -i http://localhost:4221/files/hello.txt