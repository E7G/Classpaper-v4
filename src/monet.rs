use image::{DynamicImage, ImageBuffer, Rgb, GenericImageView};
use palette::{Hsl, IntoColor, Srgb};
use std::collections::HashMap;
use std::path::Path;
use rand::Rng;

/// 图像色彩分析结果
#[derive(Debug)]
struct ImageColorAnalysis {
    warm_ratio: f32,      // 暖色调比例
    avg_lightness: f32,   // 平均亮度
    avg_saturation: f32, // 平均饱和度
}

/// 莫奈取色 - 从壁纸提取和谐配色方案
/// 
/// 基于 Material You 设计哲学，从壁纸提取主色调并生成协调的配色方案
pub struct MonetColorExtractor {
    /// 提取的主色调
    primary_color: (u8, u8, u8),
    /// 生成的配色方案
    color_scheme: ColorScheme,
}

/// 配色方案 - 包含完整的UI配色
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub primary: String,           // 主色调
    pub primary_variant: String,   // 主色调变体
    pub secondary: String,         // 辅助色
    pub secondary_variant: String, // 辅助色变体
    pub background: String,        // 背景色
    pub surface: String,           // 表面色
    pub error: String,             // 错误色
    pub warning: String,           // 警告色（重要事件）
    pub on_primary: String,        // 主色上的文字
    pub on_secondary: String,      // 辅助色上的文字
    pub on_background: String,     // 背景上的文字
    pub on_surface: String,        // 表面上的文字
    pub on_error: String,          // 错误色上的文字
    pub on_warning: String,        // 警告色上的文字
}

impl MonetColorExtractor {
    /// 从图片文件创建莫奈取色器
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("无法打开图片: {}", e))?;
        Self::from_image(&img)
    }

    /// 从图像数据创建莫奈取色器
    pub fn from_image(img: &DynamicImage) -> Result<Self, String> {
        // 1. 智能缩放 - 保持宽高比的同时确保足够的采样密度
        let (width, height) = img.dimensions();
        let scale_factor = if width * height > 500_000 { 0.25 } else { 0.5 };
        let new_width = (width as f32 * scale_factor).max(128.0).min(512.0) as u32;
        let new_height = (height as f32 * scale_factor).max(128.0).min(512.0) as u32;
        
        let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        let rgb_img = resized.to_rgb8();

        // 2. 使用改进的K-means++算法提取主要颜色
        let dominant_colors = Self::extract_dominant_colors_improved(&rgb_img, 8);
        
        // 3. 基于Material You哲学选择主色
        let primary_color = Self::select_primary_color_monet(&dominant_colors, &rgb_img);
        
        // 4. 生成符合Material You的完整配色方案
        let color_scheme = Self::generate_monet_color_scheme(primary_color);

        Ok(MonetColorExtractor {
            primary_color,
            color_scheme,
        })
    }

    /// 提取主要颜色（改进版K-means++）
    fn extract_dominant_colors_improved(img: &ImageBuffer<Rgb<u8>, Vec<u8>>, k: usize) -> Vec<(u8, u8, u8)> {
        let total_pixels = (img.width() * img.height()) as usize;
        let sample_rate = if total_pixels > 100_000 { 0.01 } else { 0.05 }; // 1-5%采样率
        let sample_interval = (1.0 / sample_rate) as usize;
        
        // 收集采样点并进行颜色量化
        let mut color_samples: Vec<(u8, u8, u8)> = Vec::new();
        let mut color_counts: HashMap<(u8, u8, u8), usize> = HashMap::new();
        
        for (i, (_, _, pixel)) in img.enumerate_pixels().enumerate() {
            if i % sample_interval == 0 {
                // 颜色量化到7位精度（减少颜色数量）
                let quantized_color = (
                    (pixel[0] >> 1) << 1,  // 丢弃最低位
                    (pixel[1] >> 1) << 1,
                    (pixel[2] >> 1) << 1,
                );
                color_samples.push(quantized_color);
                *color_counts.entry(quantized_color).or_insert(0) += 1;
            }
        }

        if color_samples.is_empty() {
            return vec![(128, 128, 128)];
        }

        // K-means++初始化 - 选择更好的初始中心点
        let mut centroids = Vec::new();
        
        // 第一个中心点：选择出现频率最高的颜色
        let first_centroid = *color_counts.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(color, _)| color)
            .unwrap_or(&(128, 128, 128));
        centroids.push(first_centroid);

        // 剩余的K-means++初始化
        for _ in 1..k.min(16) {
            let distances: Vec<(usize, f32)> = color_samples.iter()
                .enumerate()
                .map(|(i, color)| {
                    let min_dist = centroids.iter()
                        .map(|centroid| Self::color_distance(color, centroid))
                        .fold(f32::INFINITY, |a, b| a.min(b));
                    (i, min_dist * min_dist) // 平方距离
                })
                .collect();

            // 按距离加权选择下一个中心点
            let total_weight: f32 = distances.iter().map(|(_, dist)| dist).sum();
            if total_weight > 0.0 {
                let mut rng = rand::thread_rng();
                let target = rng.gen::<f32>() * total_weight;
                let mut cumulative = 0.0;
                
                for (i, &(_, weight)) in distances.iter().enumerate() {
                    cumulative += weight;
                    if cumulative >= target {
                        centroids.push(color_samples[i]);
                        break;
                    }
                }
            }
        }

        // 迭代优化（最多20次迭代或收敛）
        for _ in 0..20 {
            let mut clusters: Vec<Vec<(u8, u8, u8)>> = vec![Vec::new(); centroids.len()];
            
            // 分配样本到最近的中心点
            for color in &color_samples {
                let (_, min_idx) = centroids.iter()
                    .enumerate()
                    .map(|(i, centroid)| (Self::color_distance(color, centroid), i))
                    .fold((f32::INFINITY, 0), |(min_dist, min_idx), (dist, idx)| {
                        if dist < min_dist { (dist, idx) } else { (min_dist, min_idx) }
                    });
                clusters[min_idx].push(*color);
            }

            // 更新中心点
            let mut new_centroids = Vec::new();
            for cluster in &clusters {
                if cluster.is_empty() {
                    continue;
                }
                
                let (sum_r, sum_g, sum_b) = cluster.iter()
                    .fold((0u64, 0u64, 0u64), |(r, g, b), (cr, cg, cb)| {
                        (r + *cr as u64, g + *cg as u64, b + *cb as u64)
                    });
                
                let count = cluster.len() as u64;
                new_centroids.push((
                    (sum_r / count) as u8,
                    (sum_g / count) as u8,
                    (sum_b / count) as u8,
                ));
            }

            if new_centroids.len() == centroids.len() {
                let max_shift = centroids.iter().zip(&new_centroids)
                    .map(|(old, new)| Self::color_distance(old, new))
                    .fold(0.0f32, |a, b| a.max(b));
                
                centroids = new_centroids;
                if max_shift < 2.0 { // 收敛条件
                    break;
                }
            } else {
                centroids = new_centroids;
            }
        }

        // 按聚类大小排序返回
        let mut result: Vec<((u8, u8, u8), usize)> = centroids.into_iter()
            .map(|centroid| {
                let count = color_samples.iter()
                    .filter(|sample| Self::color_distance(sample, &centroid) < 15.0)
                    .count();
                (centroid, count)
            })
            .collect();
        
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result.into_iter().map(|(color, _)| color).collect()
    }

    /// 计算两个颜色之间的欧几里得距离
    fn color_distance(a: &(u8, u8, u8), b: &(u8, u8, u8)) -> f32 {
        let dr = (a.0 as i16 - b.0 as i16) as f32;
        let dg = (a.1 as i16 - b.1 as i16) as f32;
        let db = (a.2 as i16 - b.2 as i16) as f32;
        (dr * dr + dg * dg + db * db).sqrt()
    }

    /// 基于Material You哲学选择主色
    fn select_primary_color_monet(colors: &[(u8, u8, u8)], img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> (u8, u8, u8) {
        if colors.is_empty() {
            return (102, 126, 234); // 默认蓝色
        }

        // 计算图像的整体色彩特征
        let img_hsl = Self::analyze_image_colors(img);
        let is_mostly_warm = img_hsl.warm_ratio > 0.6;
        let avg_lightness = img_hsl.avg_lightness;
        let avg_saturation = img_hsl.avg_saturation;

        let mut best_color = colors[0];
        let mut best_score = f32::MIN;

        for &color in colors {
            let (r, g, b) = color;
            let hsl: Hsl = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color();
            
            // Material You的核心原则：
            // 1. 色彩和谐度（与整体色调一致）
            // 2. 足够的饱和度（但不过饱和）
            // 3. 适中的亮度（但考虑整体图像亮度）
            // 4. 避免过于接近极端颜色
            
            let hue_degrees = hsl.hue.into_degrees();
            let is_warm = Self::is_warm_color(hue_degrees);
            
            // 和谐度得分：与整体色调保持一致
            let harmony_score = if is_mostly_warm == is_warm { 1.0 } else { 0.6 };
            
            // 饱和度得分：Material You偏好中等饱和度
            let ideal_saturation = if avg_saturation > 0.7 { 0.7 } else { avg_saturation * 0.8 + 0.2 };
            let saturation_score = 1.0 - (hsl.saturation - ideal_saturation).abs();
            
            // 亮度得分：考虑图像整体亮度
            let ideal_lightness = if avg_lightness < 0.3 {
                0.4 // 暗图像选择稍亮的颜色
            } else if avg_lightness > 0.7 {
                0.6 // 亮图像选择稍暗的颜色
            } else {
                avg_lightness // 中等亮度图像保持中性
            };
            let lightness_score = 1.0 - (hsl.lightness - ideal_lightness).abs() * 2.0;
            
            // 极端颜色惩罚：避免过灰或过饱和
            let extreme_penalty = if hsl.saturation < 0.1 || hsl.saturation > 0.95 {
                0.5
            } else if hsl.lightness < 0.1 || hsl.lightness > 0.9 {
                0.3
            } else {
                1.0
            };
            
            // 综合得分
            let total_score = (harmony_score * 0.3 + 
                             saturation_score * 0.4 + 
                             lightness_score * 0.3) * extreme_penalty;

            if total_score > best_score {
                best_score = total_score;
                best_color = color;
            }
        }

        best_color
    }

    /// 分析图像的整体色彩特征
    fn analyze_image_colors(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> ImageColorAnalysis {
        let mut warm_colors = 0;
        let mut total_lightness = 0.0;
        let mut total_saturation = 0.0;
        let mut count = 0;

        // 采样分析（每10个像素采样1个）
        for (i, (_, _, pixel)) in img.enumerate_pixels().enumerate() {
            if i % 10 == 0 {
                let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
                let hsl: Hsl = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color();
                
                let hue_degrees = hsl.hue.into_degrees();
                if Self::is_warm_color(hue_degrees) {
                    warm_colors += 1;
                }
                
                total_lightness += hsl.lightness;
                total_saturation += hsl.saturation;
                count += 1;
            }
        }

        if count == 0 {
            return ImageColorAnalysis {
                warm_ratio: 0.5,
                avg_lightness: 0.5,
                avg_saturation: 0.5,
            };
        }

        ImageColorAnalysis {
            warm_ratio: warm_colors as f32 / count as f32,
            avg_lightness: total_lightness / count as f32,
            avg_saturation: total_saturation / count as f32,
        }
    }

    /// 判断是否为暖色调（基于色相）
    fn is_warm_color(hue_degrees: f32) -> bool {
        // 暖色调范围：红-橙-黄（0-60度，300-360度）
        hue_degrees <= 60.0 || hue_degrees >= 300.0
    }

    /// 生成符合Material You的完整配色方案
    fn generate_monet_color_scheme(primary: (u8, u8, u8)) -> ColorScheme {
        let (r, g, b) = primary;
        let primary_hsl: Hsl = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color();

        // Material You的Tonal系统：基于主色的亮度生成和谐配色
        let is_dark = Self::is_dark_color(primary);
        let primary_lightness = primary_hsl.lightness;
        
        // 生成辅助色：使用Material You的Tonal调色板方法
        let secondary_color = Self::generate_monet_secondary_color(primary, primary_hsl);
        
        // 生成变体色：基于Material You的Tonal系统
        let primary_variant = Self::adjust_lightness_monet(primary, if is_dark { 0.15 } else { -0.15 });
        let secondary_variant = Self::adjust_lightness_monet(secondary_color, if is_dark { 0.15 } else { -0.15 });

        // Material You的背景和表面色：基于亮度对比度
        let (background, surface) = if is_dark {
            // 深色主题：使用真正的Material You深色色调
            let bg_lightness = (primary_lightness + 0.05).min(0.15); // 确保足够的对比度
            let surface_lightness = (primary_lightness + 0.08).min(0.20);
            
            let bg = Self::hsl_to_rgb(primary_hsl.hue.into(), 0.05, bg_lightness);
            let surf = Self::hsl_to_rgb(primary_hsl.hue.into(), 0.03, surface_lightness);
            
            (Self::rgb_to_hex(bg), Self::rgb_to_hex(surf))
        } else {
            // 浅色主题：使用Material You浅色色调
            let bg_lightness = (primary_lightness - 0.05).max(0.95); // 确保足够的对比度
            let surface_lightness = (primary_lightness - 0.02).max(0.98);
            
            let bg = Self::hsl_to_rgb(primary_hsl.hue.into(), 0.02, bg_lightness);
            let surf = Self::hsl_to_rgb(primary_hsl.hue.into(), 0.01, surface_lightness);
            
            (Self::rgb_to_hex(bg), Self::rgb_to_hex(surf))
        };

        // 基于对比度生成文字颜色（符合WCAG标准）
        let on_primary = Self::get_contrasting_text_color(primary);
        let on_secondary = Self::get_contrasting_text_color(secondary_color);
        let on_error = "#ffffff".to_string();

        // Material You的文字层次系统
        let (on_background, on_surface) = if is_dark {
            let text_lightness = if primary_lightness > 0.7 { 0.95 } else { 0.87 };
            let _secondary_text_lightness = text_lightness * 0.7;
            
            let text_color = Self::hsl_to_rgb(primary_hsl.hue.into(), 0.05, text_lightness);
            (Self::rgb_to_hex(text_color), Self::rgb_to_hex(text_color))
        } else {
            let text_lightness = if primary_lightness < 0.3 { 0.13 } else { 0.87 };
            let _secondary_text_lightness = text_lightness * 0.6;
            
            let text_color = Self::hsl_to_rgb(primary_hsl.hue.into(), 0.05, text_lightness);
            (Self::rgb_to_hex(text_color), Self::rgb_to_hex(text_color))
        };

        // 生成警告色：基于莫奈取色逻辑的重要事件颜色
        let warning_color = Self::generate_monet_warning_color(primary, primary_hsl);

        ColorScheme {
            primary: Self::rgb_to_hex(primary),
            primary_variant: Self::rgb_to_hex(primary_variant),
            secondary: Self::rgb_to_hex(secondary_color),
            secondary_variant: Self::rgb_to_hex(secondary_variant),
            background,
            surface,
            error: Self::generate_monet_error_color(primary, is_dark),
            warning: warning_color, // 新增警告色字段
            on_primary,
            on_secondary,
            on_background,
            on_surface,
            on_error,
            on_warning: "#000000".to_string(), // 警告色上的文字统一用黑色确保可读性
        }
    }

    /// 生成Material You风格的辅助色
    fn generate_monet_secondary_color(primary: (u8, u8, u8), primary_hsl: Hsl) -> (u8, u8, u8) {
        // Material You的辅助色生成策略：
        // 1. 基于主色的色相偏移（通常15-45度）
        // 2. 保持相似的饱和度
        // 3. 调整亮度以提供足够的对比度
        
        let hue_offset = if primary_hsl.saturation > 0.6 {
            30.0 // 高饱和度：中等偏移
        } else {
            20.0 // 低饱和度：小偏移
        };
        
        let secondary_hue = (primary_hsl.hue.into_degrees() + hue_offset) % 360.0;
        let secondary_saturation = (primary_hsl.saturation * 0.9).clamp(0.2, 0.8);
        
        // 调整亮度以提供对比度
        let secondary_lightness = if Self::is_dark_color(primary) {
            (primary_hsl.lightness + 0.15).min(0.7)
        } else {
            (primary_hsl.lightness - 0.1).max(0.3)
        };
        
        let secondary = Hsl::new(secondary_hue, secondary_saturation, secondary_lightness);
        let secondary_rgb: Srgb = secondary.into_color();
        
        (
            (secondary_rgb.red * 255.0) as u8,
            (secondary_rgb.green * 255.0) as u8,
            (secondary_rgb.blue * 255.0) as u8,
        )
    }

    /// 生成Material You风格的错误色
    fn generate_monet_error_color(_primary: (u8, u8, u8), is_dark: bool) -> String {
        // Material You的错误色：基于Material Design标准，但考虑主题
        if is_dark {
            "#ff6b6b".to_string() // 稍亮的红色确保在深色背景上可见
        } else {
            "#d32f2f".to_string() // 标准的Material Design错误色
        }
    }

    /// 生成Material You风格的警告色（重要事件用）
    fn generate_monet_warning_color(primary: (u8, u8, u8), primary_hsl: Hsl) -> String {
        // Material You的警告色策略：基于主色的色相偏移到黄色/橙色区域
        // 1. 色相偏移到45-60度（黄色-橙色范围）
        // 2. 保持高饱和度确保警告效果
        // 3. 调整亮度确保在不同主题下的可见性
        // 4. 重要事件倒计时需要更亮更显眼
        
        let target_hue = 50.0; // 黄色-橙色之间的最佳警告色
        let hue_diff = (target_hue - primary_hsl.hue.into_degrees()).abs();
        
        // 如果主色已经很接近黄色，则使用橙色作为警告色避免混淆
        let final_hue = if hue_diff < 30.0 {
            target_hue + 30.0 // 偏移到橙色
        } else {
            target_hue
        };
        
        let warning_saturation = 0.9; // 增加饱和度到0.9确保更醒目
        let warning_lightness = if Self::is_dark_color(primary) {
            0.85 // 深色主题用更亮的警告色
        } else {
            0.75 // 浅色主题用更亮的警告色
        };
        
        let warning = Hsl::new(final_hue, warning_saturation, warning_lightness);
        let warning_rgb: Srgb = warning.into_color();
        
        Self::rgb_to_hex((
            (warning_rgb.red * 255.0) as u8,
            (warning_rgb.green * 255.0) as u8,
            (warning_rgb.blue * 255.0) as u8,
        ))
    }

    /// Material You风格的亮度调整
    fn adjust_lightness_monet(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
        let (r, g, b) = color;
        let hsl: Hsl = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color();
        
        // Material You的调整策略：保持饱和度，只调整亮度
        let new_lightness = (hsl.lightness + factor).clamp(0.1, 0.9);
        let new_hsl = Hsl::new(hsl.hue, hsl.saturation, new_lightness);
        let new_rgb: Srgb = new_hsl.into_color();
        
        (
            (new_rgb.red * 255.0) as u8,
            (new_rgb.green * 255.0) as u8,
            (new_rgb.blue * 255.0) as u8,
        )
    }

    /// HSL转RGB
    fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> (u8, u8, u8) {
        let hsl = Hsl::new(hue, saturation, lightness);
        let rgb: Srgb = hsl.into_color();
        (
            (rgb.red * 255.0) as u8,
            (rgb.green * 255.0) as u8,
            (rgb.blue * 255.0) as u8,
        )
    }

    /// 获取对比度足够的文字颜色（符合WCAG标准）
    fn get_contrasting_text_color(background: (u8, u8, u8)) -> String {
        let lightness = Self::calculate_relative_lightness(background);
        if lightness > 0.5 {
            "#000000".to_string() // 深色文字
        } else {
            "#ffffff".to_string() // 浅色文字
        }
    }

    /// 计算相对亮度（WCAG标准）
    fn calculate_relative_lightness(color: (u8, u8, u8)) -> f32 {
        let (r, g, b) = color;
        // WCAG相对亮度计算公式
        let rs = (r as f32 / 255.0).powf(2.2);
        let gs = (g as f32 / 255.0).powf(2.2);
        let bs = (b as f32 / 255.0).powf(2.2);
        0.2126 * rs + 0.7152 * gs + 0.0722 * bs
    }

    /// 调整颜色亮度
    fn adjust_lightness(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
        let (r, g, b) = color;
        let hsl: Hsl = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color();
        let new_lightness = (hsl.lightness + factor).clamp(0.0, 1.0);
        let new_hsl = Hsl::new(hsl.hue, hsl.saturation, new_lightness);
        let new_rgb: Srgb = new_hsl.into_color();
        
        (
            (new_rgb.red * 255.0) as u8,
            (new_rgb.green * 255.0) as u8,
            (new_rgb.blue * 255.0) as u8,
        )
    }

    /// 判断是否为暗色（使用WCAG标准）
    fn is_dark_color(color: (u8, u8, u8)) -> bool {
        Self::calculate_relative_lightness(color) < 0.5
    }

    /// RGB转十六进制
    fn rgb_to_hex(color: (u8, u8, u8)) -> String {
        format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2)
    }

    /// 获取配色方案
    pub fn get_color_scheme(&self) -> &ColorScheme {
        &self.color_scheme
    }

    /// 获取主色调
    pub fn get_primary_color(&self) -> (u8, u8, u8) {
        self.primary_color
    }

    /// 生成CSS变量字符串，可直接注入到前端
    pub fn generate_css_variables(&self) -> String {
        let scheme = &self.color_scheme;
        let background_rgb = Self::extract_rgb_from_hex(&scheme.background);
        let _surface_rgb = Self::extract_rgb_from_hex(&scheme.surface);
        let primary_rgb = Self::extract_rgb_from_hex(&scheme.primary);
        let secondary_rgb = Self::extract_rgb_from_hex(&scheme.secondary);
        let warning_rgb = Self::extract_rgb_from_hex(&scheme.warning); // 使用莫奈取色的警告色
        let success_rgb = Self::extract_rgb_from_hex(&scheme.primary); // 使用主色作为成功色
        
        format!(
            r#"
:root {{
  /* 莫奈取色 - 动态配色方案 - 统一命名规范 */
  --primary-color: {};
  --primary-variant: {};
  --secondary-color: {};
  --secondary-variant: {};
  --background-color: {};
  --surface-color: {};
  --error-color: {};
  --warning-color: {};
  --text-on-primary: {};
  --text-on-secondary: {};
  --text-on-background: {};
  --text-on-surface: {};
  --text-on-error: {};
  --text-on-warning: {};
  
  /* 兼容原有变量名 - 保持现有样式工作 */
  --monet-primary: {};
  --monet-primary-variant: {};
  --monet-secondary: {};
  --monet-secondary-variant: {};
  --monet-background: {};
  --monet-surface: {};
  --monet-error: {};
  --monet-warning: {};
  --monet-on-primary: {};
  --monet-on-secondary: {};
  --monet-on-background: {};
  --monet-on-surface: {};
  --monet-on-error: {};
  --monet-on-warning: {};
  
  /* RGB值用于玻璃拟态效果 */
  --primary-rgb: {};
  --secondary-rgb: {};
  --glass-bg-rgb: {};
  --warning-rgb: {};
  --success-rgb: {};
  
  /* 渐变效果 */
  --primary-gradient: linear-gradient(135deg, var(--primary-color) 0%, var(--primary-variant) 100%);
  --secondary-gradient: linear-gradient(135deg, var(--secondary-color) 0%, var(--secondary-variant) 100%);
  --warning-gradient: linear-gradient(135deg, var(--warning-color) 0%, var(--secondary-variant) 100%);
  --success-gradient: linear-gradient(135deg, var(--primary-color) 0%, var(--primary-variant) 100%);
}}

/* 基础颜色应用 */
body {{
  background-color: var(--background-color) !important;
  color: var(--text-on-background) !important;
}}

/* 玻璃拟态卡片 - 保持原有视觉效果 */
.glass-card {{
  background: rgba(var(--glass-bg-rgb), var(--glass-opacity, 0.1)) !important;
  border-color: rgba(var(--glass-bg-rgb), var(--glass-border-opacity, 0.2)) !important;
  backdrop-filter: blur(var(--blur-strength, 20px)) !important;
}}

/* 按钮系统 - 保持玻璃拟态效果 */
.btn {{
  background: rgba(var(--glass-bg-rgb), var(--glass-opacity, 0.1)) !important;
  border-color: rgba(var(--glass-bg-rgb), var(--glass-border-opacity, 0.2)) !important;
  color: var(--text-on-surface) !important;
  backdrop-filter: blur(10px) !important;
}}

.btn-primary {{
  background: var(--primary-gradient) !important;
  color: var(--text-on-primary) !important;
  border-color: transparent !important;
}}

.btn-secondary {{
  background: rgba(var(--surface-color-rgb), 0.8) !important;
  color: var(--text-on-surface) !important;
  border-color: rgba(var(--glass-bg-rgb), 0.1) !important;
}}

/* 课程卡片 - 保持原有状态指示器 */
.course-card {{
  background: rgba(var(--glass-bg-rgb), var(--glass-opacity, 0.1)) !important;
  border-color: rgba(var(--glass-bg-rgb), var(--glass-border-opacity, 0.2)) !important;
}}

.course-card.current {{
  border-left-color: var(--primary-color) !important;
  background: rgba(var(--primary-rgb), 0.2) !important;
}}

.course-card.next {{
  border-left-color: var(--secondary-color) !important;
  background: rgba(var(--secondary-rgb), 0.2) !important;
}}

.course-card.past {{
  border-left-color: var(--text-secondary) !important;
  opacity: 0.5 !important;
}}

/* 进度条 - 保持动画效果 */
.progress-fill {{
  background: linear-gradient(90deg, var(--primary-color), var(--primary-variant), var(--primary-color)) !important;
  background-size: 200% 100% !important;
}}

/* 事件卡片 - 保持警告色调 */
.event-item {{
  background: linear-gradient(135deg, rgba(var(--warning-rgb), 0.12), rgba(var(--warning-rgb), 0.03)) !important;
  border-color: rgba(var(--warning-rgb), 0.15) !important;
}}

.countdown-number {{
  color: var(--secondary-color) !important;
}}

/* 文字颜色 - 保持层次感 */
.title {{
  color: var(--text-on-background) !important;
}}

.subtitle {{
  color: var(--text-secondary) !important;
}}

.course-name {{
  color: var(--text-on-background) !important;
}}

.course-time {{
  color: var(--text-secondary) !important;
}}
"#,
            // 标准变量名
            scheme.primary, scheme.primary_variant, scheme.secondary, scheme.secondary_variant,
            scheme.background, scheme.surface, scheme.error, scheme.warning,
            scheme.on_primary, scheme.on_secondary, scheme.on_background, scheme.on_surface, scheme.on_error, scheme.on_warning,
            // 兼容变量名
            scheme.primary, scheme.primary_variant, scheme.secondary, scheme.secondary_variant,
            scheme.background, scheme.surface, scheme.error, scheme.warning,
            scheme.on_primary, scheme.on_secondary, scheme.on_background, scheme.on_surface, scheme.on_error, scheme.on_warning,
            // RGB值
            primary_rgb, secondary_rgb, background_rgb, warning_rgb, success_rgb
        )
    }

    /// 从十六进制提取RGB值
    fn extract_rgb_from_hex(hex: &str) -> String {
        if hex.starts_with('#') && hex.len() == 7 {
            let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
            format!("{}, {}, {}", r, g, b)
        } else {
            "255, 255, 255".to_string()
        }
    }

    /// 从十六进制颜色判断是否为暗色（供外部调用）
    pub fn is_dark_color_from_hex(hex: &str) -> bool {
        if hex.starts_with('#') && hex.len() == 7 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[1..3], 16),
                u8::from_str_radix(&hex[3..5], 16),
                u8::from_str_radix(&hex[5..7], 16),
            ) {
                return Self::calculate_relative_lightness((r, g, b)) < 0.5;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_extraction() {
        // 创建一个测试图片
        let mut img = ImageBuffer::new(100, 100);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([100, 150, 200]); // 蓝色调
        }
        
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let extractor = MonetColorExtractor::from_image(&dynamic_img).unwrap();
        let scheme = extractor.get_color_scheme();
        
        assert!(scheme.primary.contains("#"));
        assert!(scheme.secondary.contains("#"));
    }
}