# Elusiv Merkle Tree Assignment


## Algorithm

## Optimizations & Readability
Often there are certain tradeoffs between optimizing code to be
the most efficient and having code that's readable and maintainable.
I try to have all three but it's always we can have that. Readability
is also a subjective thing but there's a difference between code that is
a subjective preference and code that is generally considered awful.(like putting everything in one lib.rs or over abstraction).

### Algorithmic Improvements
1. Since the leaves are pre-filled with defaults, when inserting we can't simply push the hash of the data so the naive approach is to loop until you find a default value and replace it. However this O(n<sup>2</sup>) so instead I made the tree to save the current index it should add the leaf at and this helps keep track of which index to replace the leaf. This makes the efficiency closer to O(1) and resulted in a mean time ~800 micro secs faster.

<!-- ![Before](images/1-before.png)
![After](images/1-after.png) -->
<figure>
  <img src="images/1-before.png" alt="Before">
  <figcaption>Before</figcaption>
</figure>
<figure>
  <img src="images/1-after.png" alt="After">
  <figcaption>After</figcaption>
</figure>
2. Pre-allocating vectors since we know the capacity of the tree also saves unnessary allocations
in the heap saving us from overhead of making syscalls(due to context switching). 

### Readability Tradeoffs
1. When calculating the level length or height of the tree we have the choice between using floating point calculations like this:
```rs
 (current_level_len as f64 / 2.0).ceil() as usize
```
or integer arithmatic like this:
```rs
if current_level_len % 2 == 0 {
    current_level_len / 2
} else {
    (current_level_len + 1) / 2
}
```

While floating point calculations do require a bit more compute, the performance
difference in our specific use case (especially when tree h <= 10) is
marginal. Therefore I personally feel since we are also returning a `usize` it's better
to rely on the second option.

