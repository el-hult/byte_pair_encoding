A naive byte pair encoding implementation in rust.

To test it out, run

```bash
git clone https://github.com/el-hult/byte_pair_encoding.git
cd byte_pair_encoding
echo "This is the text that the BPE will train on. We wish to have at least three 'is' so it becomes merged." > train.txt
echo "This text file will be encoded using the trained BPE." > test.txt
cargo run
```

This will 
1. Train a BPE on `train.txt`. It generates tokens until the least used token is used less than 3 times. Report how many tokens there are in the trained BPE tokenizer.
1. Encode and decode `train.txt` to the terminal with colorized token boundaries.
1. Serialize the trained BPE to a file `bpe.bin`.
1. Load the BPE from the file `bpe.bin`.
1. Encode and decode `test.txt` to the terminal with colorized token boundaries.

The code is separated into a library `lib.rs` that someone else could use as they like, and a simple `main.rs` that demonstrates how to use the library.

### Insights from implementation
Some things came to mind while implementing this. There is a rich litterature on the topic that I did not consult, so my insights below might be a bit naive.

#### General ideas
Imagine there are `m` byte-pair reduction rules, and a string to encode is `n` long.
 - Encoding a string must be done in the order of the most frequent pairs in the corpus. This requires me to make `m` passes over a string that is `n` long, giving complexity `O(mn)`, which is bad. One can do some splitting up and parallelizing the work, but as long as it is done on the CPU, it will not be great on longer pieces of text.
 - The decoding I have implemented is quite slow. It uses a `Vec<Vec<u8>>`, so I suppose the indirection for lookups can be bad for cache locality. If I was sure that null byte was reserved, I could let each token decode into a c-string capped at certain max size (getting a `Vec<[MAX;u8]>` instead of `Vec<Vec<u8>>` as the data structure for decoding tokens). One less pointer to follow in each decoding step.
 - My BPE does not do anythin related to unicode boundaries. Doing the BPE on byte level or character level is a tradeoff and I am not sure about what is best. When colorizing the output I check the token boundaries at each token and this is quite expensive. I could speed up the decoding if not trying to utf8 decode each token.
 - Dictionary size determination is a design choice. It feels like there is some minimum description length kind of problem going on here. Should we add tokens until message+dictionary size starts to grow again? Adding a token to the dictionary means removing `k` pairs, reducing the tokenized message length by `k` tokens. But I add an encoding rule to the BPE Tokenizer (space cost of `3` tokens) and a decoding rule (costs `1+l/2` tokens where `l` is the length of the decoded token in bytes). So as long if `3 + 1 + l/2 < k`, we should add the token? 
 - The decoder is now a `Vec<Vec<u8>>`, but we *could* do recursive decoding, applying a `HashMap<(Token,Token),Token>` many times instead. More space efficient. Much slower.

 #### Implications for LLM applications
 - In LLM implementations, the dictionary size should also consider the embedding space dimensionality. If that space is small, we cannot have a lot of orthogonal directions, and that could be a reason to reduce the dictionary size. But more tokens means shorter messages, and a transformer architecture suffers less from its quadratic time complexity. There is a architecture dependant trade off here.
 - Presence of input sequences guaranteed to never be reduced further into compound tokens (e.g. full stops if we don't want tokens that span multiple sentences, or  `\`\`\`` if we don't want tokens that span both text and code blocks) can be used to segment the input text snippets that can be encoded in parallel. The choice of such boundaries are context dependant. Are we doing a model for natural language or for some structured format?
 - Control tokens like <|human|> and <|img|> and so on, often found in huggingface implementations of LLMs are easy to add as a single token in the decoder table after training. How should we deal with them in the encoding step? If we want to encode them as input text we will have a in-band-singalling problem. I think that the tokenizer should get the input in some mixed format instead, so that control tokens can be inserted in the first step, then the text input i converted to bytes, and the control tokens and the byte tokens are combined before running the BPE.
 - Unicode character boundary aligned tokens could be better for decoding, since that means an LLM cannot generate invalid utf8 strings. However, the number of raw tokens increase, as we would need one token per unicode code point. I would probably not find a corpus where they all are used either. And the embedding table needs a embedding vector per code point. It seems to me that such an encoding is inefficient, and byte level encoding is better.