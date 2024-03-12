0. utility 
    * raw_vec - wrapper around raw vec pointer with external capacity & len managment
    * primes_u16 - all primes numbers up to u16::MAX
    * disposable - trait that could be implemented by structure which required some end of the frame disposing 

0. primitive types - minimal implementation 
    * ivec - fully lock free vec, 
    * sparse_array - single thread hash map but wiht bitfields 
    * sync_sparse_array - 

1. Stories - abstraction over 1 or more primitive structures that for 


## 11.3.2024

1. How we wants to keep updates 
    1. set ofcourse drop previous version and insert new one, insert do nothiung,
        those changes could be detected simple via comparison of current bits with updated bits? 
        NOPE because updates bitts doens't describe existence of entity, so we need to know what with entity is it exists then it probably inserter/updated if deleted then we don't want to touch it until next insert or fully deletion based on enitty 
        the problem ins't updates bits the problem that deleted state required some info, that we currently don't have,  the only way to tell if entity has an component is via directly accessing entity ? if this is what we want ? even thoue how to ddetect compoennt deletion with single bit? we could copy it, but then we don't know what changed 
    2. delete probably shod't delete immideately so script won't fail, the problem isn't information 

    1. we could directly set that something is deleted on main bits, then when we pocess updates if something deleted and updated at the same time we could treat as deleted, if something existed in update bits, but non exists in main bits then it's mean it inserted (this is also mega safe because we don't need to sync those vars), if value 0 then it non updated, if 1 1 then it udpated 

        but question if we really want delay insertion ????? check for example visibliy component we don't want it to appear after the end, then question is we want to detect update and inserion differently ? potentially yes, then we there should exists breakpoints that apply changes 

    00 - value not exists and not updated
    10 - value exists but not updated
    01 - value updated and deleted
    11 - value updated (potentially inserted) we don't now 

The advantage of inserion detection that we don't need for addition comparison, the advantage of deletion detection that we don't know 

The last question is timers if they should be part of current architecture so user will be abble to use them just by specifying max-lifetime: 
1. entity lifetime is managed by entity_id component which basically means that when it deleted, we also must delete everething else related to component
2. other component lifetime is stored near them and means, when they inserted or udpated we must re-set timer 
3. all timers is lazy which means that when we acces entity we must check timer and if it out od bounds we could disable entity (and we also must track this change ), this must work even for immutable access 
4. links timers is different and storead as atomic in hashmap, they not autodeleted, they will autoinvalidated, but this also means that yhey could store only weak refrence to entity 
5. the question how timers should be integreted into ? does we need every sparse array with timer or just some of them and how they should be checked ? while accesing to entity and comparision wtih some global variable (basically how metrics works)

it's non productive to store timers for non components, becauyse most the things is could be scheduled via entities timers, 

then my question is it possible to create bucket that covers both time & non-time access ? and about dropping value, because re-inserting value required to access to component entity to check if entity has this component in past, this may hugely slowdown insertion, but it's still needed to do in both cases, because if we know that entity delete we must re-insert it, then question maybe we need to care some info about entity ? :D like probably with timer + state / deleted / inserted etc ... it's slowdown iteration for sure but may imrpove other things  

1. Create examples with iteration over udpates and resettings updates info
    1. The teoreticall problem only with how to allow user iterate only over update entities
    1. Then we could combine them with simple queries and allow user to works with udpates without additional syntax

2. Make set_in_place non blockable by providing collisions list
    * This + Scheduling with mut access should probably fully solve problem with possible infinity locking
    * What about scripting, if they also allowed only to insert entire component, or we could update only single field
        * This probably required non-blocking iterations, i.e bits should be loaded with Atomic Ordering isntead of keeping raw references, then there won't exist method to lock something for longer duration and we could allow update_in_place(fn) that update single field
            * Related question is what apbout direct links to component, I suppose they should update with same update_in_place or event store entire entity instead of component!!!!

3. Finish example when we get iterator of entities for different archs that need to set component or even zero sized component and update them each cycle
    * The question for this only is set_in_place the best sollution, because of locking/unlocking or we could make it better ?
        * We could do this in two steps
            * Reset Vec + Aggregate + Sort In Place by arch + id
            * Technically we could create batch insert wich in case of close entities will be effective, but if we take in account that all entities is differnt archs then we don't save a lot of time 
                * V1. Access entity -> Access Prototype -> Access Needed bucket
                * V2. Bin search arch (baiscally leanear seca) -> Probably access needed buckeet directly if we don't need min/max

1. add booleans if components was changed & if sparse array was changed
1.1 Apply both changes on BucketDrop if changes was detected 
1.2 Changes detection must be done via changes bits 
1.3 We don't need bits as raw pointer because changes applied to them could be done from mutliple threads, but hten we have a poblem with insertion, what will happens if we set bits but update not applied yet, this probably not possible with accquiting ordering
1.4 is strong ref to componnets & storing ptr to sparse array + bucket idx is better than storing separate refs to bucket & bits and 

2. Introduce update tracker array 
3. Introduce changes iteration 
4. Introduce collection changes tracker
5. Introduce changes application stage