use plotters::prelude::*;
use rand::Rng;

// A function to compute the equal interval breaks
pub fn equal_interval(values: &[f64], nb_class: u32) -> Vec<f64> {
    let sorted_values = {
        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted_values
    };
    let min = sorted_values.first().unwrap();
    let max = sorted_values.last().unwrap();
    let interval = (*max - *min) / nb_class as f64;
    let mut breaks = Vec::new();
    let mut val = *min;
    for _ in 0..(nb_class + 1) {
        breaks.push(val);
        val += interval;
    }
    {
        let last = breaks.last_mut().unwrap();
        *last = *max;
    }
    breaks
}

// A Gaussian kernel function for kernel density estimation
fn gaussian_kernel(distance: f64, bandwidth: f64) -> f64 {
    (-0.5 * (distance / bandwidth).powi(2)).exp()
        / (bandwidth * (2.0 * std::f64::consts::PI).sqrt())
}

// A function to generate random points
fn generate_random_points(num_points: usize, range_min: i32, range_max: i32) -> Vec<(i32, i32)> {
    let mut rng = rand::thread_rng(); // Get a random number generator
    (0..num_points)
        .map(|_| {
            (
                rng.gen_range(range_min..=range_max),
                rng.gen_range(range_min..=range_max),
            )
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate random points
    let input_points = generate_random_points(100, 0, 1000);

    // Find the bounding box of the input points
    // to create a regular grid
    let (min_x, max_x) = input_points
        .iter()
        .fold((i32::MAX, i32::MIN), |(min, max), &(x, _)| {
            (min.min(x) - 1, max.max(x) + 1)
        });
    let (min_y, max_y) = input_points
        .iter()
        .fold((i32::MAX, i32::MIN), |(min, max), &(_, y)| {
            (min.min(y) - 1, max.max(y) + 1)
        });

    // Create a regular grid
    let x_step = 0.5;
    let y_step = 0.5;
    let width = max_x - min_x;
    let height = max_y - min_y;
    let nb_points_x = width as f64 / x_step;
    let nb_points_y = height as f64 / y_step;

    let grid: Vec<(f64, f64)> = (0..nb_points_y as u32)
        .flat_map(|j| {
            (0..nb_points_x as u32).map(move |i| {
                (
                    min_x as f64 + i as f64 * x_step,
                    min_y as f64 + j as f64 * y_step,
                )
            })
        })
        .collect();

    // Interpolate the input points to the regular grid
    // using the gaussian kernel density estimation
    let bandwidth = 75.0;
    let kde_values: Vec<f64> = grid
        .iter()
        .map(|&(gx, gy)| {
            input_points
                .iter()
                .map(|&(x, y)| {
                    let distance = ((gx - x as f64).powi(2) + (gy - y as f64).powi(2)).sqrt();
                    gaussian_kernel(distance, bandwidth)
                })
                .sum::<f64>()
                / input_points.len() as f64
        })
        .collect();

    // Compute the threshold values for the isolines
    let threshold_values = equal_interval(&kde_values, 6);

    // Feed the interpolated grid to the contour crate
    let contour_builder =
        contour::ContourBuilder::new(nb_points_x as u32, nb_points_y as u32, true)
            .x_origin(min_x as f64)
            .y_origin(min_y as f64)
            .x_step(x_step)
            .y_step(y_step);

    // Compute the isolines
    let lines = contour_builder.lines(&kde_values, &threshold_values)?;

    // Plot the points and the resulting isolines
    let root = BitMapBackend::new("plot.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Build the chart
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Plotting points and contour lines",
            ("sans-serif", 22).into_font(),
        )
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    // Draw points
    chart.draw_series(PointSeries::of_element(
        input_points,
        2,
        &BLACK,
        &|coord, size, color| EmptyElement::at(coord) + Circle::new((0, 0), size, color.filled()),
    ))?;

    // Get a color palettes from colorbrewer
    let ramp_orange = colorbrewer::get_color_ramp(colorbrewer::Palette::Blues, lines.len() as u32)
        .ok_or("Palette not found")?;

    // Draw isolines
    for (i, line) in lines.iter().enumerate() {
        // The line geometry is a MultiLineString, access it with the geometry method
        let geometry = line.geometry();
        // Get the color for the isoline
        let custom_color = RGBColor(ramp_orange[i].r, ramp_orange[i].g, ramp_orange[i].b);
        // Iterate over the lines in the MultiLineString
        for line_string in geometry {
            // Iterate over the coordinates in the LineString
            let points: Vec<(i32, i32)> = line_string
                .into_iter()
                .map(|coord| (coord.x as i32, coord.y as i32))
                .collect();
            chart.draw_series(LineSeries::new(points, &custom_color))?;
        }
    }

    Ok(())
}
