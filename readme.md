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


### Insights from implementation
Some things came to mind while implementing this. Imagine there are `m` byte-pair reduction rules, and a string to encode is `n` long.

 - Encoding a string must be done in the order of the most frequent pairs in the corpus. This requires me to make `m` passes over a string that is `n` long, giving complexity `O(mn)`, which is bad. One can do some splitting up and parallelizing the work, but as long as it is done on the CPU, it will not be great on longer pieces of text.
 - The decoding scheme I have put forward is very slow. Since each token can represent an arbitrary amount of bytes, it is hard to optimize the decoder. I use vectors right now, and that is also bad for locality. If I was sure that null byte was reserved, I could let each token decode into a c-string capped at certain max size, and I would have a `Vec<[MAX;u8]>` instead of `Vec<Vec<u8>>`. One less pointer to follow in each decoding step.
 - A hot path for decoding bytes without looking up in the encoding vector might be good. On small corpuses this would matter. Not on a large one where almost never raw bytes are used.
 - My BPE does not deal with unicode boundaries in general. I only care about them when making colorized output. Not toknizing along unicode boundaries means that any generative language model may produce non-utf8 characters. That is bad. On the other hand, if the BPE has to respect characters, we have a space of reserved tokens that is the space of all unicode (currently only raw bytes are reserved), forcing me to use `u64` tokens instead of `u32`. Overall, doing the BPE on byte level or character level is a tradeoff and I am not sure about what is best.
 - The colorized output showing token boundares is really nice. But it makes the code slower, as I must utf8 encode every token separately.


### General ideas
This seems to be a quite nice scheme not only for LLM tokenization, but also for compression in general. Some ideas for future investigations.

 - How large dictionary should I use? It seems there is some minimum description length kind of problem going on here. Should we add tokens until message+dictionary size starts to grow again? As of writing, adding a token to the dictionary means removing `k` pairs, reducing the tokenized message length by `k` tokens. But I add a encoding rule (`3` tokens) and a decoding rule (`1+l/2` tokens where `l` is the length of the decoded token in bytes). So as long if `3 + 1 + l/2 < k`, we should add the token? If `l` is long, we could also have a recursive decoding step as well, trading processing power for serialized memory footprint.
 - What is the relation to huffman encoding? There, the codes are variable length, wehereas here all tokens have a fixed size. Why is this bad for LLMs?
 - In LLM implementations, the dictionary size should also consider the embedding space dimensionality. If that space is small, we cannot have a lot of orthogonal directions, and that could be a reason to reduce the dictionary size. But more tokens means shorter messages, and the transformer architecture suffers less from its quadratic complexity. So there is a tradeoff here as well. And it is architecture dependent.
 - Precense of input sequences guaranteed to never be reduced further into compound tokens (e.g. full stops if we don't want tokens that span multiple sentences, or  `\`\`\`` if we don't want tokens that span both text and code blocks) can be used to segment the input text snippets that can be encoded in parallel. The choice of such boundaries are context dependant. Are we doing a model for natural language or for some structured format?
 - Control tokens like <|human|> and <|img|> and so on, often found in huggingface implementations of llms, is easy to add as a single token in the decoder table. but how to deal with them in the input step? If we want to encode them as input text we will have a in-band-singalling problem. I think that the tokenizer should get the input in some mixed format instead, so that control tokens can be inserted in the first step, where I convert all bytes to unreduced tokens.