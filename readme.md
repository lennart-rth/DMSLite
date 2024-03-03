# llm classification
- von allen typen immer alle bekannten schon bestehenden typen in der promt mitgeben.
- give every knwon institution and type of document as example in the prompt. say: einen dieser zuordnen oder neue categorie erstellen.
- aus welchem jahr stammt diese dokument. wenn nicht gegeben return "-"
- von welcher institution kommt das dokument? eine von den hier oder unbekannt, dann geben einen neuen namen.
- type of dokument (neu wenn unbekannt.)
- generiere eine kurzen title der das dokument minimal beschreibt

# settings
- wie viele der seiten als doc content nutzen
- feste categorient zum einordnen nennen
- pfad in dem die documente gespeichert werden
- mount pfad für den symlink-tree

# indexing and tree generation
## main table
- id (primary)
- upload date
- relative filepath

## document table (fuzzy searchable)
- id (primary)
- document content
- document buzzwords
- document summary


# Userstories:
- must consum docs from a folder
- must make docs searchable for content, title -respond with
- must make docs sortable by creation date
- must be able to create filesystem with the content as symbolic links fast. (dynamicly at browse time?)


1. consume  2. search <phrase> , <date>  3. list   4. parse symtree

2. show results for phrase
    1. open in appropiate software   2. show in symtree


```ollama create doc_title_generator -f doc_title_generator ```