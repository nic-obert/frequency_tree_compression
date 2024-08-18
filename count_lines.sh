#!/bin/bash
find . -name *.rs | xargs wc -l | sort -nr
