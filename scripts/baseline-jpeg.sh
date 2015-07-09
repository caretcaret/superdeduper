#!/bin/sh

find $1 -type f -name "*.jpg*" -exec jpegtran -perfect -copy all -outfile "{}" "{}" \;

for file in $(find $1 -type f -name "*.png*"); do
  if [ $(identify -format '%m' $file) == JPEG ]; then
    jpegtran -perfect -copy all "$file" > "${file}.jpg" && rm $file;
  fi
done

