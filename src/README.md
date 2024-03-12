1. the idea is to make `BucketRefMut` has references to both bitfields:
    1. bits that describes entity existence 
    2. bits that describes if entity was updated

    ↓

    3. if bits that describes if entity was updated is null pointer then ignore this operation
       otherwise process also should update this bits
    
    ↓ 

    4. to make both implmentation compatibily bucket creation must be done from extended &(bits, ptr, bits)
    
    ↓

    5. this means that we shoud cast short represenation into full representation by adding null pointer to the end of tuple before creating bucket
