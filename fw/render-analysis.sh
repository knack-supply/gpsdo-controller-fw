#!/bin/sh

cargo test --tests -- control::tests::

NBCONVERT="docker run --rm -v $(pwd)/sim:/sim jupyter/scipy-notebook:6c3390a9292e jupyter nbconvert"

$NBCONVERT --to notebook --inplace --execute /sim/data/simulation-analysis.ipynb
rm -Rf sim/simulation-analysis_files
$NBCONVERT --to markdown --no-input /sim/data/simulation-analysis.ipynb --output-dir /sim/simulation-analysis --output index.md
