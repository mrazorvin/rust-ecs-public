Current questions:
## Patches & Links & ChunkStorage
* Patches must be stored outside of ComponentChunk and support dense form
  * Dense operations should support atomic add/remove
  * Contaienr structure should support atomic resizing
  * Patch indexes will be stored into SpraseArray and will be created alongside with main component
  * Patch indexes will contains aotmic value that could be used as lock for atomic add/remove
  * All patch operations is asyncrnyhous and must be done with custom API 
  * Pathes must supoort iteration at the end of cyle to apply their changes 
  * Patches must support fast clearing at the end of cycle
  * Dense patch storage must conain only releavat  patches for current cycle

* ID's will be stored in separte collection than data, because of possible future changes a nd better cache comptibilities
  * Sotring for such structure could be copied from shipyard library 
  * By support ting soerting we must support cross compomnents fsorting 
  * We must support iteration over specific sroted components collection 

* Links must be stored outside of ComponentChunk and support dense form
  * Links is always 2 way 
  * Links component in dense form represent real data i.e etity_id1 <-> entity_id2
  * Links components in sparse form represen existence of links
  * Links storage contains dense and sparse form 
  * Linsk storage sparse form point to first link in dense array 
  * Links storage dense form contain 2w 2dimensial linked list (for both A <-> B, B <-> C, C <-> B) 
  * Links storage must be in form slot map or similar structure, to reduce re-indexing during adding/removing links
  * All link operation is synchronious and must be done with custom API
  * All links presence must be done via iteration over entire lis

* To support archetypes with large collection of components ECS should dinammicaly increase archetype capacicty,
  the best way to do that is to add u16 offset for entity_id, and make ECS know archetypes capacity entities, i.e common formula for calculation enity archetype = (entity_arch - entity_arch_offset)

* All dense data will be stored in single vec instead of ComponentChunk, since the only reason to store data in 
  Chunk was better caching, which could be simulated with ordering, i.e time to time components could be reordered based on archetype or any other attribute
  
  * This storage form allow us:
    * Clear data by dropping single vector
    * Faster iteration over all existet components 
    * Possibility to sort based on how often entity was used by application 
    * Smaller `ComponentChunk` for faster access to coponent's id's
    * We could sort all component at once 
    * We don't have problem for Componmnetn with more than 1000 archetypes 
    * Links is global instead of local 