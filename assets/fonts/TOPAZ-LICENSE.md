# Police Topaz (Kickstart 3.0, style "OS 2.0+")

Ces fichiers (`topaz_ks30_regular.ttf`, `topaz_ks30_bold.ttf`) sont dérivés des
données bitmap `amiga-ks30-topaz-08.yaff` du projet
[hoard-of-bitfonts](https://github.com/robhagemans/hoard-of-bitfonts) de Rob
Hagemans, qui préserve les polices bitmap de la ROM Kickstart 3.0 (le style
Topaz utilisé à partir d'AmigaOS 2.0+/3.x, plus arrondi que la version 1.3
d'origine).

Conversion effectuée localement pour ce projet :
1. `amiga-ks30-topaz-08.yaff` → BDF 8×16 (doublage vertical) via
   [monobit](https://github.com/robhagemans/monobit).
2. BDF → TTF vectoriel via
   [BitsNPicas](https://github.com/kreativekorp/bitsnpicas) (même outil que
   celui utilisé par le projet
   [topaz-unicode](https://gitlab.com/Screwtapello/topaz-unicode) pour la
   police Kickstart 1.3 utilisée précédemment dans ce projet).
3. Correction des métadonnées `unitsPerEm`/ascender/descender (bug de
   BitsNPicas laissant `unitsPerEm` à 0).

## Licence

Selon `hoard-of-bitfonts` (CC0 pour le travail de préservation) :

> In the USA, bitmap typefaces are not copyrightable. In the UK, the 25-year
> term of copyright for these typefaces has expired.

Vous devez vérifier les règles applicables dans votre propre juridiction.
