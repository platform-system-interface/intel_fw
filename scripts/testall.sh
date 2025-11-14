#!/bin/sh

#me_cleaner -O ./fixtures/x230_cleaned_simple.rom ./fixtures/x230.rom
cargo run --release -- me clean -O x230_clean.rom ./fixtures/x230.rom
#me_cleaner -k -O ./fixtures/x230_cleaned_k.rom ./fixtures/x230.rom
cargo run --release -- me clean -k -O x230_clean_s.rom ./fixtures/x230.rom
#me_cleaner -s -O ./fixtures/x230_cleaned_s.rom ./fixtures/x230.rom
cargo run --release -- me clean -s -O x230_clean_s.rom ./fixtures/x230.rom
#me_cleaner -S -O ./fixtures/x230_cleaned_S.rom ./fixtures/x230.rom
cargo run --release -- me clean -S -O x230_clean_S.rom ./fixtures/x230.rom
#me_cleaner -r -O ./fixtures/x230_cleaned_reloc.rom ./fixtures/x230.rom
cargo run --release -- me clean -r -O x230_clean_reloc.rom ./fixtures/x230.rom
#me_cleaner -r -t -O ./fixtures/x230_cleaned_trunc.rom ./fixtures/x230.rom
cargo run --release -- me clean -r -t -O x230_clean_trunc.rom ./fixtures/x230.rom
#me_cleaner -w MFS -O ./fixtures/x230_cleaned_w_MFS.rom ./fixtures/x230.rom
cargo run --release -- me clean -w MFS -O x230_clean_w_MFS.rom ./fixtures/x230.rom
#me_cleaner -b MFS -O ./fixtures/x230_cleaned_b_MFS.rom ./fixtures/x230.rom
cargo run --release -- me clean -b MFS -O x230_clean_b_MFS.rom ./fixtures/x230.rom
#me_cleaner -w EFFS -O ./fixtures/x230_cleaned_w_EFFS.rom ./fixtures/x230.rom
cargo run --release -- me clean -w EFFS -O x230_clean_w_EFFS.rom ./fixtures/x230.rom
#me_cleaner -b EFFS -O ./fixtures/x230_cleaned_b_EFFS.rom ./fixtures/x230.rom
cargo run --release -- me clean -b EFFS -O x230_clean_b_EFFS.rom ./fixtures/x230.rom

#me_cleaner -O ./fixtures/x270_cleaned_simple.rom ./fixtures/x270.rom
cargo run --release -- me clean -O x270_clean.rom ./fixtures/x270.rom
#me_cleaner -k -O ./fixtures/x270_cleaned_k.rom ./fixtures/x270.rom
cargo run --release -- me clean -k -O x270_clean_k.rom ./fixtures/x270.rom
#me_cleaner -s -O ./fixtures/x270_cleaned_s.rom ./fixtures/x270.rom
cargo run --release -- me clean -s -O x270_clean_s.rom ./fixtures/x270.rom
#me_cleaner -S -O ./fixtures/x270_cleaned_S.rom ./fixtures/x270.rom
cargo run --release -- me clean -S -O x270_clean_S.rom ./fixtures/x270.rom
#me_cleaner -r -O ./fixtures/x270_cleaned_reloc.rom ./fixtures/x270.rom
cargo run --release -- me clean -r -O x270_clean_reloc.rom ./fixtures/x270.rom
#me_cleaner -r -t -O ./fixtures/x270_cleaned_trunc.rom ./fixtures/x270.rom
cargo run --release -- me clean -r -t -O x270_clean_trunc.rom ./fixtures/x270.rom
#me_cleaner -w MFS -O ./fixtures/x270_cleaned_w_MFS.rom ./fixtures/x270.rom
cargo run --release -- me clean -w MFS -O x270_clean_w_MFS.rom ./fixtures/x270.rom
#me_cleaner -b MFS -O ./fixtures/x270_cleaned_b_MFS.rom ./fixtures/x270.rom
cargo run --release -- me clean -b MFS -O x270_clean_b_MFS.rom ./fixtures/x270.rom
#me_cleaner -w EFFS -O ./fixtures/x270_cleaned_w_EFFS.rom ./fixtures/x270.rom
cargo run --release -- me clean -w EFFS -O x270_clean_w_EFFS.rom ./fixtures/x270.rom
#me_cleaner -b EFFS -O ./fixtures/x270_cleaned_b_EFFS.rom ./fixtures/x270.rom
cargo run --release -- me clean -b EFFS -O x270_clean_b_EFFS.rom ./fixtures/x270.rom

./scripts/bdiffstat x230_clean.rom fixtures/x230_cleaned_simple.rom
./scripts/bdiffstat x230_clean_k.rom fixtures/x230_cleaned_k.rom
./scripts/bdiffstat x230_clean_s.rom fixtures/x230_cleaned_s.rom
./scripts/bdiffstat x230_clean_S.rom fixtures/x230_cleaned_S.rom
./scripts/bdiffstat x230_clean_reloc.rom fixtures/x230_cleaned_reloc.rom
./scripts/bdiffstat x230_clean_trunc.rom fixtures/x230_cleaned_trunc.rom
./scripts/bdiffstat x230_clean_w_MFS.rom fixtures/x230_cleaned_w_MFS.rom
./scripts/bdiffstat x230_clean_b_MFS.rom fixtures/x230_cleaned_b_MFS.rom
./scripts/bdiffstat x230_clean_w_EFFS.rom fixtures/x230_cleaned_w_EFFS.rom
./scripts/bdiffstat x230_clean_b_EFFS.rom fixtures/x230_cleaned_b_EFFS.rom

./scripts/bdiffstat x270_clean.rom fixtures/x270_cleaned_simple.rom
./scripts/bdiffstat x270_clean_k.rom fixtures/x270_cleaned_k.rom
./scripts/bdiffstat x270_clean_s.rom fixtures/x270_cleaned_s.rom
./scripts/bdiffstat x270_clean_S.rom fixtures/x270_cleaned_S.rom
./scripts/bdiffstat x270_clean_reloc.rom fixtures/x270_cleaned_reloc.rom
./scripts/bdiffstat x270_clean_trunc.rom fixtures/x270_cleaned_trunc.rom
./scripts/bdiffstat x270_clean_w_MFS.rom fixtures/x270_cleaned_w_MFS.rom
./scripts/bdiffstat x270_clean_b_MFS.rom fixtures/x270_cleaned_b_MFS.rom
./scripts/bdiffstat x270_clean_w_EFFS.rom fixtures/x270_cleaned_w_EFFS.rom
./scripts/bdiffstat x270_clean_b_EFFS.rom fixtures/x270_cleaned_b_EFFS.rom
