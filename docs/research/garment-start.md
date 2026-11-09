# Starting a job from the garment (issue #56)

Status: first version 2026-11-09. Revision history at the end.

## What was decided

A screen printer starts with the item, its colour and where the print goes, not a page. So
`screen.newJob` takes those three choices and makes a document whose artboard is the print area.
The sheet size is chosen later, with the films.

## The numbers are rules of thumb, not facts

Every number below is a common practitioner default that the user can change in the window. **They
are not yet sourced to a published table**: this note was written without checking a vendor chart,
and the owner's issue gave left chest of about 4 in wide and full front of about 12 × 14 in. Before
a public release, check each against a public size guide and cite it here (an open question, not a blocker).

| Item | Location | Width × height (in) | Where it came from |
|------|----------|---------------------|--------------------|
| T-shirt, hoodie | Left chest | 4 × 4 | the issue (about 4 in wide); height chosen square |
| T-shirt | Full front, full back | 12 × 14 | the issue |
| Hoodie | Full front, full back | 11 × 13 | chosen a little smaller for the pocket seam; unsourced |
| T-shirt, hoodie | Sleeve | 3.5 × 3.5 | unsourced |
| T-shirt, hoodie | Oversized | 16 × 20 | unsourced; a large platen size |
| Tote bag | Front | 10 × 12 | unsourced |
| Tote bag | Oversized | 12 × 14 | unsourced |
| Poster / paper | Custom | 18 × 24 | a common poster size |
| Other | Custom | 12 × 12 | placeholder |

## Underbase by garment colour

Dark garments get a white underbase flashed before the colours, light ones print straight on
(the practice is described in `silkscreen-landscape.md`, "Underbase, trap, choke and registration").
The switch is CIE L\* below 60 for the garment colour (`sc_core::garment::DARK_BELOW_L`). 60 is a
chosen line, not a measured one: red, mid green and royal blue shirts sit close to it and print
shops differ, so the Underbase box in the Screen Print panel can always be flipped and an explicit
`underbase` parameter always wins.

## The drawings

The garment outlines (T-shirt, hoodie hood, tote with straps, paper) are original polygons made in
code (`sc_core::garment::Mockup`), not traced from any product photo or another application's
mockups. They carry no brand. The swatch names and colours are our own.

## Left for later

* Customer proof export (the mockup with the art as a PNG or PDF).
* A mockup that shows the art itself on the garment at its true place (today the art sits on the
  artboard, which is the print area, over the garment drawing).
* Sleeve on the other side, long sleeves, youth and ladies sizes.

## Revision history

* 2026-11-09: first version with issue #56.
