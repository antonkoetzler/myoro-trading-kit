# Custom Strategy Extensions

Strategies live in the `strategies/` directory. Two formats are supported: **TOML** (field expressions) and **Rhai** (scripted logic).

---

## TOML Strategy

Create `strategies/<name>.toml`:

```toml
[strategy]
id = "xg_value"
name = "xG Value Finder"
domain = "sports"          # all | crypto | sports | weather
enabled = true
description = "Finds value on home teams with strong xG stats"

[strategy.risk]
kelly_fraction = 0.20      # fraction of Kelly criterion to use
min_edge = 0.08            # minimum required edge to generate a signal

[logic]
side = "yes"               # yes | no
mode = "all"               # all = AND, any = OR

[[logic.conditions]]
expr = "home_xg_per90 > 1.8"

[[logic.conditions]]
expr = "home_win_rate - market_yes_price > min_edge"

[logic.edge]
expr = "home_win_rate - market_yes_price"

[logic.confidence]
expr = "if home_win_rate > 1.0 { 1.0 } else { home_win_rate }"
```

Expressions are **Rhai** syntax. All `DataContext` fields become variables.

---

## Available DataContext Fields by Domain

### Sports
| Field | Type | Description |
|-------|------|-------------|
| `home_win_rate` | f64 | Historical home win % |
| `market_yes_price` | f64 | Current market YES price (0–1) |
| `home_xg_per90` | f64 | Expected goals per 90 min |
| `away_xg_per90` | f64 | Away team xG per 90 |
| `over_2_5_rate` | f64 | Historical over-2.5-goals % |
| `btts_rate` | f64 | Both-teams-to-score % |

### Crypto
| Field | Type | Description |
|-------|------|-------------|
| `price` | f64 | Current price |
| `volume_24h` | f64 | 24h volume |
| `lag_pct` | f64 | Lag vs reference market (%) |
| `market_yes_price` | f64 | Polymarket probability |

### Weather
| Field | Type | Description |
|-------|------|-------------|
| `temp_max` | f64 | Forecast max temperature |
| `temp_min` | f64 | Forecast min temperature |
| `precip` | f64 | Precipitation (mm) |
| `wind_max` | f64 | Max wind speed (km/h) |
| `market_yes_price` | f64 | Polymarket probability |

---

## Rhai Script Strategy

For complex logic, use a `.rhai` script:

```toml
[strategy]
id = "smart_arb"
name = "Smart Arb"
domain = "crypto"
engine = "rhai"
script = "strategies/smart_arb.rhai"
```

The script receives all DataContext fields as global variables and must return a boolean (`true` = signal).

---

## Reloading Strategies

Strategies are loaded at startup. Restart the terminal to pick up changes.
