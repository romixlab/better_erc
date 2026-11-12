# Places the test footprints of Test.pretty on a four-layer board at several positions, rotations and sides and
# prints it as KiCad saves a board: the board copies erc_kicad's footprint test compares with the library files.
# Run through KiCad's container image: `just footprint-fixture`.
import pcbnew

LIB = "/w/Test.pretty"
# (footprint, x, y, rotation, back, flip left-right)
PLACES = [
    ("Asym_SOT", 20, 20, 0, False, False),
    ("Asym_SOT", 30, 20, 90, False, False),
    ("Asym_SOT", 40, 20, 30, False, False),
    ("Asym_SOT", 20, 30, 0, True, False),
    ("Asym_SOT", 30, 30, 180, True, False),
    ("Asym_SOT", 40, 30, -45, True, True),
    ("Mixed_Pads", 20, 50, 0, False, False),
    ("Mixed_Pads", 35, 50, 270, False, False),
    ("Mixed_Pads", 50, 50, 135, False, False),
    ("Mixed_Pads", 20, 65, 0, True, False),
    ("Mixed_Pads", 35, 65, 90, True, False),
    ("Mixed_Pads", 50, 65, 20, True, True),
]

board = pcbnew.BOARD()
board.SetCopperLayerCount(4)
for i, (name, x, y, rot, back, left_right) in enumerate(PLACES):
    fp = pcbnew.FootprintLoad(LIB, name)
    fp.SetReference(f"U{i + 1}")
    fp.SetPosition(pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(y)))
    board.Add(fp)
    fp.SetOrientationDegrees(rot)
    if back:
        direction = pcbnew.FLIP_DIRECTION_LEFT_RIGHT if left_right else pcbnew.FLIP_DIRECTION_TOP_BOTTOM
        fp.Flip(fp.GetPosition(), direction)
pcbnew.SaveBoard("/tmp/placed.kicad_pcb", board)
print(open("/tmp/placed.kicad_pcb").read(), end="")
