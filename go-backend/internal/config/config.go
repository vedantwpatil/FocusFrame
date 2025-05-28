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
