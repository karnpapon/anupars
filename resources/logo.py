def bresenham(x0, y0, x1, y1):
  points = []
  dx = abs(x1 - x0)
  dy = abs(y1 - y0)
  x, y = x0, y0
  sx = 1 if x1 > x0 else -1
  sy = 1 if y1 > y0 else -1

  if dx >= dy:
    err = dx // 2
    while x != x1:
      points.append((x, y))
      err -= dy
      if err < 0:
        y += sy
        err += dx
      x += sx
    points.append((x, y))
  else:
    err = dy // 2
    while y != y1:
      points.append((x, y))
      err -= dx
      if err < 0:
        x += sx
        err += dy
      y += sy
    points.append((x, y))

  return points


BLOCK = 40
NX = 9
NY = 8
THICKNESS = 2
WIDTH = BLOCK * NX
HEIGHT = BLOCK * NY


def expand(cells, dc, dr, t, nx, ny):
  """Expand a set of cells by t-1 extra blocks in direction (dc, dr), clamped to grid."""
  result = set(cells)
  for col, row in cells:
    for k in range(1, t):
      nc, nr = col + dc * k, row + dr * k
      if 0 <= nc < nx and 0 <= nr < ny:
        result.add((nc, nr))
  return result


left_diag = bresenham(0, NY - 1, NX - 1, 0)
right_vert = bresenham(NX - 1, NY - 1, NX - 1, 0)
tail = bresenham(NX // 2, NY // 2, NX - 1, NY - 1)
thick_diag = expand(left_diag, 1, 0, THICKNESS, NX, NY)
thick_vert = expand(right_vert, -1, 0, THICKNESS, NX, NY)
thick_tail = expand(tail, -1, 0, THICKNESS, NX, NY)

lit = thick_diag | thick_vert | thick_tail

lines = []
lines.append(
    f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}">')
# draw lit cells only — background is transparent
for row in range(NY):
  for col in range(NX):
    if (col, row) in lit:
      x = col * BLOCK
      y = row * BLOCK
      lines.append(
          f'  <rect x="{x}" y="{y}" width="{BLOCK}" height="{BLOCK}" fill="#000000"/>')

lines.append('</svg>')

out = '\n'.join(lines)
path = './resources/logo.svg'
with open(path, 'w') as f:
  f.write(out)
