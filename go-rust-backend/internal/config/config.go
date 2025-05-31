package config

type Config struct {
	Effects struct {
		Blur struct {
			Enabled bool
			Radius  int
		}
		Zoom struct {
			Enabled bool
			Factor  float64
		}
		Follow struct {
			Enabled bool
			Window  float64 // Window size in seconds before and after click
		}
	}
	Processing struct {
		Parallel bool
		Workers  int
	}
	Recording struct {
		TargetFPS int
		OutputDir string
	}
}

func NewConfig() *Config {
	return &Config{
		Effects: struct {
			Blur struct {
				Enabled bool
				Radius  int
			}
			Zoom struct {
				Enabled bool
				Factor  float64
			}
			Follow struct {
				Enabled bool
				Window  float64
			}
		}{
			Blur: struct {
				Enabled bool
				Radius  int
			}{
				Enabled: true,
				Radius:  5,
			},
			Zoom: struct {
				Enabled bool
				Factor  float64
			}{
				Enabled: true,
				Factor:  1.5,
			},
			Follow: struct {
				Enabled bool
				Window  float64
			}{
				Enabled: true,
				Window:  1.0, // 1 second window before and after click
			},
		},
		Processing: struct {
			Parallel bool
			Workers  int
		}{
			Parallel: true,
			Workers:  4,
		},
		Recording: struct {
			TargetFPS int
			OutputDir string
		}{
			TargetFPS: 60,
			OutputDir: "output",
		},
	}
}
