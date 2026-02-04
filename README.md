# file-fingerprinting

This project is a simple program that computes checksums for all of the files
in a directory (including subdirectories) and writes them to a file. The goal is
to be able to find files that are likely duplicates of each other.

> [!NOTE]
> Because the goal is for the checksum computation to be quick (this
project is meant to handle millions of files relatively quickly), it uses CRC64
as the checksum algorithm. This is not a cryptographically secure hash, but it is
fast. Unfortunately, it has the downsides of two files with the same checksum
not being guaranteed duplicates.
>
> To handle this, I had also written tools that
would compute the SHA256 checksum for files with the same CRC64 checksum.
Unfortunately, and becuase this project is old, I don't have the code for those
tools anymore. Either way, it's a good proof of concept.
