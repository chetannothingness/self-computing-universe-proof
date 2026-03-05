#!/usr/bin/env python3
"""
Export kernel exo-patch output to visualization JSON.

Usage:
    1. cargo run -- exo-patch --output /tmp/exo
    2. python3 visualization/export_exo.py /tmp/exo/addons/TOE_REAL
    3. Open visualization/real-universe.html and load the generated exo_viz.json
"""
import csv
import json
import re
import sys
import os

def parse_stars_csv(csv_path):
    """Parse TOE_ExoHosts.csv. Columns: Name, RA(arcsec), Dec(arcsec), Dist(pc), SpType, AppMagV."""
    stars = []
    with open(csv_path, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            try:
                ra_deg = float(row['RA']) / 3600.0      # arcsec → degrees
                dec_deg = float(row['Dec']) / 3600.0     # arcsec → degrees
                dist_pc = float(row['Dist'])             # already parsecs
                mag = float(row['AppMagV']) if row['AppMagV'] else None
            except (ValueError, KeyError):
                continue

            stars.append({
                'name': row['Name'],
                'ra': round(ra_deg, 6),
                'dec': round(dec_deg, 6),
                'dist': round(dist_pc, 4),
                'spectral': row.get('SpType', '').strip(),
                'mag': round(mag, 3) if mag is not None else None,
            })
    return stars

def parse_planets_sc(sc_path):
    """Parse TOE_ExoPlanets.sc (SpaceEngine format)."""
    planets = []
    with open(sc_path, 'r') as f:
        text = f.read()

    # Split into planet blocks
    blocks = re.split(r'(?=^Planet )', text, flags=re.MULTILINE)
    for block in blocks:
        block = block.strip()
        if not block.startswith('Planet'):
            continue

        name_m = re.search(r'Planet\s+"([^"]+)"', block)
        parent_m = re.search(r'ParentBody\s+"([^"]+)"', block)
        if not name_m or not parent_m:
            continue

        planet = {
            'name': name_m.group(1),
            'parent': parent_m.group(1),
        }

        for field, key in [('Mass', 'mass'), ('Radius', 'radius'),
                           ('Period', 'period'), ('SemiMajorAxis', 'sma'),
                           ('Eccentricity', 'ecc'), ('Inclination', 'inc')]:
            m = re.search(rf'{field}\s+([\d.eE+-]+)', block)
            if m:
                planet[key] = round(float(m.group(1)), 6)

        planets.append(planet)
    return planets

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 export_exo.py <addon_dir> [output.json]")
        print("  addon_dir: e.g., /tmp/exo/addons/TOE_REAL")
        sys.exit(1)

    addon_dir = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else 'exo_viz.json'

    csv_path = os.path.join(addon_dir, 'catalogs', 'stars', 'TOE_ExoHosts.csv')
    sc_path = os.path.join(addon_dir, 'catalogs', 'planets', 'TOE_ExoPlanets.sc')
    merkle_path = os.path.join(addon_dir, 'proof', 'merkle.json')

    if not os.path.exists(csv_path):
        print(f"Error: {csv_path} not found. Run: cargo run -- exo-patch --output /tmp/exo")
        sys.exit(1)

    stars = parse_stars_csv(csv_path)
    planets = parse_planets_sc(sc_path)

    # Load proof metadata
    proof = {}
    if os.path.exists(merkle_path):
        with open(merkle_path) as f:
            proof = json.load(f)

    # Stats
    spec_count = sum(1 for s in stars if s['spectral'])
    mass_count = sum(1 for p in planets if p.get('mass'))
    period_count = sum(1 for p in planets if p.get('period'))

    data = {
        'proof': proof,
        'stars': stars,
        'planets': planets,
        'stats': {
            'total_stars': len(stars),
            'total_planets': len(planets),
            'stars_with_spectral': spec_count,
            'planets_with_mass': mass_count,
            'planets_with_period': period_count,
        },
    }

    with open(output_path, 'w') as f:
        json.dump(data, f, separators=(',', ':'))

    size_kb = os.path.getsize(output_path) / 1024
    print(f"Exported {len(stars)} stars, {len(planets)} planets → {output_path} ({size_kb:.0f} KB)")
    print(f"Proof: merkle_root={proof.get('merkle_root', 'N/A')[:16]}...")

if __name__ == '__main__':
    main()
