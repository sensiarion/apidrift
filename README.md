# Yet another one openapi diff tool yopendiff

This tool is intended to help developers explore api changes in their application in most short and understandable way.

## Why i want yet another one tool

I really love [oasdiff](https://github.com/oasdiff/oasdiff) and use it in my work projects. But we have some troubles
with it.

### It is too much noise information

See example https://html-preview.github.io/?url=https://github.com/oasdiff/oasdiff/blob/main/examples/changelog.html

It's displaying all quite well, but when a look at real example on my project, i can see following.

![img.png](docs/img.png)

And about 5 screens of same change above.

### Endpoint oriented diff

My projects - mainly relay on fastapi, with highly reusable pydantic schemas. It leads to broadly changes on endpoints
with 1 line of code. To handle changes properly i need model oriented design, not routes. First of all we tracking model
changes and affecting routes, and than tracking changes on routes by itself.

I want to group changes by single model and not create 20 screen report only for single enum change.

### Resume

oasdiff - still amazing instrument, that i love to use, but I want more expressive tool to speedup changes tracking.

Yes, I can attempt to create custom theme to oasdiff and work with it, but it's not what i want. So we making this

## Target

I want to create diff tool, that:

- focuses on most expressive way to represent changes for humans
    - helps developers handle changes in types (changelog is mostly for frontend, which generates it's data types on
      openapi schema)
- blazingly fast (thx to rust)
- pre built binary for ci use
    - with simple install via curl/wget
- structured diff format (json)
- zero dependency pretty html format
    - with separated models changes and routes changes and grouping
- track changes by level, as oasdiff do (track breaking changes)
- accepts both yaml and json format

## Work steps

- [ ] parse json openapi
  - [ ] structures to parsed data
- [ ] parse multiple openapi files
- [ ] base definition for change rules (breaking, warning, non-breaking)
- [ ] base definition for change (model change, route change)

- [ ] track model changes
- [ ] track routes changes
- [ ] bind model changes to route changes

- [ ] changes to json
- [ ] changes to html
  - [ ] describe future html format (most expressive and readable)
  - [ ] prettify html
  - [ ] interactive elements to html? (grouping collapsed)

### prettifying
- [ ] human readable errors (wrong file format, incorrect openapi)
- [ ] parallel comparison run
  - [ ] will require to build dep tree or locks, to prevent miltiple parsing on recursive

### not important

- [ ] auth change/server params
- [ ] version change tracking
- [ ] headers tracking
- [ ] no unwrap without handling