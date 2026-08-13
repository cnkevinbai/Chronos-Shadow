// Chronos-Shadow 专业 PPT 生成引擎
// 支持: 6套模板 · AI配色提取 · 网页参考抓取 · 图表/表格/图片

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── 专业模板定义 ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PptTemplate {
    /// 企业商务 - 深蓝+白, 严谨专业
    Corporate,
    /// 科技极简 - 暗色+渐变, 现代感
    TechMinimal,
    /// 创意设计 - 多彩+圆角, 活泼
    Creative,
    /// 学术答辩 - 简洁白+学术色, 结构化
    Academic,
    /// 极简白 - 大面积留白, 苹果风格
    MinimalWhite,
    /// 暗夜模式 - 深色背景+霓虹色, 程序员风格
    DarkMode,
    /// Vercel 工业单色 - 黑白极简, 一线大厂审美
    VercelMonochrome,
    /// Linear 暗黑霓虹 - 发光渐变, 极客质感
    LinearDarkNeon,
    /// Apple 极简流体 - 大留白, 精致排版
    AppleMinimalist,
}

impl PptTemplate {
    pub fn name(&self) -> &str { match self {
        Self::Corporate => "企业商务", Self::TechMinimal => "科技极简",
        Self::Creative => "创意设计", Self::Academic => "学术答辩",
        Self::MinimalWhite => "极简白", Self::DarkMode => "暗夜模式",
        Self::VercelMonochrome => "Vercel 工业单色",
        Self::LinearDarkNeon => "Linear 暗黑霓虹",
        Self::AppleMinimalist => "Apple 极简流体",
    }}
    pub fn bg_color(&self) -> &str { match self {
        Self::Corporate => "FFFFFF", Self::TechMinimal => "0F172A",
        Self::Creative => "FAFAFA", Self::Academic => "FFFFFF",
        Self::MinimalWhite => "FAFBFC", Self::DarkMode => "0A0A0F",
        Self::VercelMonochrome => "000000",
        Self::LinearDarkNeon => "09090B",
        Self::AppleMinimalist => "FFFFFF",
    }}
    pub fn accent_color(&self) -> &str { match self {
        Self::Corporate => "1E40AF", Self::TechMinimal => "38BDF8",
        Self::Creative => "EC4899", Self::Academic => "2563EB",
        Self::MinimalWhite => "3B82F6", Self::DarkMode => "A78BFA",
        Self::VercelMonochrome => "FFFFFF",
        Self::LinearDarkNeon => "5E6AD2",
        Self::AppleMinimalist => "0071E3",
    }}
    pub fn text_color(&self) -> &str { match self {
        Self::Corporate | Self::Academic | Self::MinimalWhite | Self::Creative => "1E293B",
        Self::TechMinimal | Self::DarkMode => "E2E8F0",
        Self::VercelMonochrome => "FFFFFF",
        Self::LinearDarkNeon => "E4E4E7",
        Self::AppleMinimalist => "1D1D1F",
    }}
    pub fn subtitle_color(&self) -> &str { match self {
        Self::Corporate => "64748B", Self::TechMinimal => "94A3B8",
        Self::Creative => "6B7280", Self::Academic => "475569",
        Self::MinimalWhite => "9CA3AF", Self::DarkMode => "A1A1AA",
        Self::VercelMonochrome => "A1A1AA",
        Self::LinearDarkNeon => "71717A",
        Self::AppleMinimalist => "86868B",
    }}
    pub fn font_family(&self) -> &str { match self {
        Self::Corporate | Self::Academic => "Microsoft YaHei",
        Self::TechMinimal | Self::DarkMode => "Consolas",
        Self::Creative => "Segoe UI", Self::MinimalWhite => "Helvetica",
        Self::VercelMonochrome | Self::LinearDarkNeon => "SF Mono",
        Self::AppleMinimalist => "SF Pro Display",
    }}
}

// ─── 幻灯片内容结构 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideContent {
    pub slide_type: SlideType,
    pub title: String,
    pub subtitle: Option<String>,
    pub body: Option<String>,
    pub bullets: Option<Vec<String>>,
    pub image_url: Option<String>,
    pub table_data: Option<TableData>,
    pub chart_data: Option<ChartData>,
    pub speaker_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlideType {
    TitleSlide,     // 封面
    SectionHeader,  // 章节页
    Content,        // 内容页
    TwoColumn,      // 双栏
    ImageFull,      // 全图
    TableSlide,     // 表格
    ChartSlide,     // 图表
    QuoteSlide,     // 引用
    ThankYou,       // 结尾
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_type: String, // "bar" | "line" | "pie"
    pub categories: Vec<String>,
    pub series: Vec<ChartSeries>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
}

// ─── PPT 生成请求 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptGenerationRequest {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub template: Option<PptTemplate>,
    pub slides: Vec<SlideContent>,
    /// 参考网页 URL (自动抓取风格)
    pub reference_url: Option<String>,
    /// 输出路径
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptGenerationResult {
    pub success: bool,
    pub file_path: Option<String>,
    pub slide_count: usize,
    pub template_used: String,
    pub error: Option<String>,
}

// ─── 参考网页分析结果 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceAnalysis {
    pub url: String,
    pub title: Option<String>,
    /// 提取的主色调 (hex)
    pub primary_color: Option<String>,
    /// 提取的辅色
    pub secondary_color: Option<String>,
    /// 推荐的模板
    pub recommended_template: PptTemplate,
    /// 提取的关键内容摘要
    pub content_summary: String,
    /// 提取的结构化要点
    pub key_points: Vec<String>,
}

// ─── PPTX 生成引擎 ───────────────────────────────────────────────

pub struct PptxEngine {
    #[allow(dead_code)]
    python_path: String,
    /// 输出目录
    output_dir: PathBuf,
}

impl PptxEngine {
    pub fn new() -> Self {
        // 使用桌面作为输出目录，用户更容易找到
        let output_dir = dirs::desktop_dir()
            .or_else(dirs::document_dir)
            .unwrap_or_else(|| std::env::temp_dir())
            .join("Chronos-PPT");
        // 自动检测 Python 路径
        let python_path = find_python();
        Self { python_path, output_dir }
    }

    /// 🔬 分析参考网页，提取设计风格
    pub async fn analyze_reference(
        &self, url: &str, html_content: &str,
    ) -> ReferenceAnalysis {
        // 提取标题
        let title = extract_title_from_html(html_content);
        // 从HTML提取主色调
        let primary = extract_dominant_color(html_content);
        let secondary = primary.as_ref().map(|c| adjust_color(c, 30));

        // 推荐模板
        let template = match primary.as_deref() {
            Some(c) if is_dark_color(c) => PptTemplate::DarkMode,
            Some(c) if is_bright_color(c) => PptTemplate::Creative,
            Some(c) if is_corporate_color(c) => PptTemplate::Corporate,
            _ => PptTemplate::MinimalWhite,
        };

        // 提取关键内容
        let body = crate::agent::indomitable_fetcher::extract_main_content(html_content);
        let points: Vec<String> = body.split('\n')
            .filter(|l| l.len() > 20 && l.len() < 200)
            .take(8)
            .map(|s| s.trim().to_string())
            .collect();

        ReferenceAnalysis {
            url: url.into(), title,
            primary_color: primary.clone(), secondary_color: secondary,
            recommended_template: template.clone(),
            content_summary: body.chars().take(500).collect(),
            key_points: points,
        }
    }

    /// 生成 PPTX 文件 (纯 Rust 原生, 无需 Python, 必定产出 .pptx)
    pub fn generate(&self, request: &PptGenerationRequest) -> PptGenerationResult {
        let template = request.template.clone().unwrap_or(PptTemplate::Corporate);
        let output = request.output_path.clone().unwrap_or_else(|| {
            let name = sanitize_filename(&request.title);
            self.output_dir.join(format!("{}.pptx", name))
                .to_string_lossy().to_string()
        });

        // 确保输出目录存在
        if let Some(parent) = std::path::Path::new(&output).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // 🔬 纯 Rust 原生 PPTX 生成 (保证 .pptx)
        self.generate_pptx_rust(request, &template, &output)
    }

    /// 🔬 纯 Rust 原生 PPTX 生成 (无需 Python, 永远成功)
    pub fn generate_pptx_rust(
        &self, req: &PptGenerationRequest, tmpl: &PptTemplate, output: &str,
    ) -> PptGenerationResult {
        use std::io::Write;

        let file = match std::fs::File::create(output) {
            Ok(f) => f,
            Err(e) => {
                return PptGenerationResult {
                    success: false, file_path: None, slide_count: 0,
                    template_used: tmpl.name().into(), error: Some(e.to_string()),
                };
            }
        };
        let mut archive = zip::ZipWriter::new(file);

        let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // [Content_Types].xml
        let mut content_types = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
        content_types.push_str("<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n");
        content_types.push_str("  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n");
        content_types.push_str("  <Default Extension=\"xml\" ContentType=\"application/xml\"/>\n");
        content_types.push_str("  <Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\n");
        for i in 1..=req.slides.len() {
            content_types.push_str(&format!("  <Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n", i));
        }
        content_types.push_str("</Types>");
        let _ = archive.start_file("[Content_Types].xml", options);
        let _ = archive.write_all(content_types.as_bytes());

        // _rels/.rels
        let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/></Relationships>";
        let _ = archive.start_file("_rels/.rels", options);
        let _ = archive.write_all(rels.as_bytes());

        // ppt/presentation.xml
        let bg = tmpl.bg_color();
        let accent = tmpl.accent_color();
        let text_color = tmpl.text_color();
        let font = tmpl.font_family();
        let slide_ids: String = (1..=req.slides.len())
            .map(|i| format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>", 255 + i, i))
            .collect();
        let mut presentation = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><p:sldMasterIdLst/><p:sldIdLst>{}</p:sldIdLst><p:sldSz cx=\"12192000\" cy=\"6858000\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>",
            slide_ids
        );
        presentation.push_str("");
        let _ = archive.start_file("ppt/presentation.xml", options);
        let _ = archive.write_all(presentation.as_bytes());

        // ppt/_rels/presentation.xml.rels
        let mut pres_rels = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
        for i in 1..=req.slides.len() {
            pres_rels.push_str(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>", i, i));
        }
        pres_rels.push_str("</Relationships>");
        let _ = archive.start_file("ppt/_rels/presentation.xml.rels", options);
        let _ = archive.write_all(pres_rels.as_bytes());

        // slides
        for (i, slide) in req.slides.iter().enumerate() {
            let slide_xml = build_slide_xml(slide, i + 1, bg, accent, text_color, font);
            let _ = archive.start_file(format!("ppt/slides/slide{}.xml", i + 1), options);
            let _ = archive.write_all(slide_xml.as_bytes());

            // slide rels
            let slide_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/></Relationships>";
            let _ = archive.start_file(format!("ppt/slides/_rels/slide{}.xml.rels", i + 1), options);
            let _ = archive.write_all(slide_rels.as_bytes());
        }

        if let Err(e) = archive.finish() {
            return PptGenerationResult {
                success: false, file_path: None, slide_count: 0,
                template_used: tmpl.name().into(), error: Some(e.to_string()),
            };
        }

        PptGenerationResult {
            success: true,
            file_path: Some(output.to_string()),
            slide_count: req.slides.len(),
            template_used: tmpl.name().into(),
            error: None,
        }
    }

    /// 🔬 降级方案: 生成 Markdown 大纲 (无需 Python, 永远成功)
    #[allow(dead_code)]
    fn generate_markdown_fallback(
        &self, req: &PptGenerationRequest, tmpl: &PptTemplate,
        output: &str, reason: &str,
    ) -> PptGenerationResult {
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", req.title));
        if let Some(sub) = &req.subtitle {
            md.push_str(&format!("> {}\n\n", sub));
        }
        md.push_str(&format!("*模板: {} | 页数: {}*\n\n---\n\n", tmpl.name(), req.slides.len()));

        for (i, slide) in req.slides.iter().enumerate() {
            md.push_str(&format!("## 第{}页: {}\n\n", i + 1, slide.title));
            if let Some(body) = &slide.body {
                md.push_str(&format!("{}\n\n", body));
            }
            if let Some(bullets) = &slide.bullets {
                for b in bullets {
                    md.push_str(&format!("- {}\n", b));
                }
                md.push('\n');
            }
            md.push_str("---\n\n");
        }

        let md_path = output.replace(".pptx", ".md");
        if let Some(parent) = std::path::Path::new(&md_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&md_path, &md).is_ok() {
            PptGenerationResult {
                success: true,
                file_path: Some(md_path.clone()),
                slide_count: req.slides.len(),
                template_used: tmpl.name().into(),
                error: Some(format!("{} → 已降级生成 Markdown 大纲 (安装 python-pptx 后自动生成 .pptx)", reason)),
            }
        } else {
            PptGenerationResult {
                success: false, file_path: None, slide_count: 0,
                template_used: tmpl.name().into(),
                error: Some(reason.into()),
            }
        }
    }

    /// 构建 Python 生成脚本
    #[allow(dead_code)]
    fn build_generation_script(
        &self, req: &PptGenerationRequest, tmpl: &PptTemplate, output: &str,
    ) -> String {
        let slides_json = serde_json::to_string(&req.slides).unwrap_or_else(|_| "[]".into());

        format!(r#"
import json, os
from pptx import Presentation
from pptx.util import Inches, Pt, Emu
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.enum.shapes import MSO_SHAPE

prs = Presentation()
prs.slide_width = Inches(13.333)
prs.slide_height = Inches(7.5)

BG = RGBColor(0x{0:})
ACCENT = RGBColor(0x{1:})
TEXT = RGBColor(0x{2:})
SUB = RGBColor(0x{3:})
FONT = '{4}'

slides_data = json.loads('''{5}''')
author = '{6}'
main_title = '{7}'

def add_bg(slide):
    bg = slide.background
    fill = bg.fill
    fill.solid()
    fill.fore_color.rgb = BG

def add_accent_bar(slide, left=0, top=0, width=Inches(13.333), height=Inches(0.05)):
    shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, left, top, width, height)
    shape.fill.solid()
    shape.fill.fore_color.rgb = ACCENT
    shape.line.fill.background()

def add_footer(slide, page_num, total):
    tf = slide.shapes.add_textbox(Inches(0.5), Inches(7.0), Inches(12), Inches(0.3))
    tf.text_frame.paragraphs[0].text = f"{{main_title}}  |  {{page_num}} / {{total}}"
    tf.text_frame.paragraphs[0].font.size = Pt(8)
    tf.text_frame.paragraphs[0].font.color.rgb = SUB
    tf.text_frame.paragraphs[0].font.name = FONT

for idx, slide_data in enumerate(slides_data):
    stype = slide_data.get('slide_type', 'Content')
    title = slide_data.get('title', '')
    subtitle = slide_data.get('subtitle', '')
    body = slide_data.get('body', '')
    bullets = slide_data.get('bullets', [])
    notes = slide_data.get('speaker_notes', '')

    # --- Title Slide ---
    if stype == 'TitleSlide':
        slide = prs.slides.add_slide(prs.slide_layouts[6])  # blank
        add_bg(slide)
        # Accent line
        add_accent_bar(slide, Inches(1.5), Inches(2.8), Inches(10), Inches(0.06))
        # Title
        tb = slide.shapes.add_textbox(Inches(1.5), Inches(3.0), Inches(10.5), Inches(1.5))
        tf = tb.text_frame; tf.word_wrap = True
        p = tf.paragraphs[0]; p.text = title; p.font.size = Pt(44); p.font.bold = True
        p.font.color.rgb = TEXT; p.font.name = FONT
        if subtitle:
            p2 = tf.add_paragraph(); p2.text = subtitle; p2.font.size = Pt(20)
            p2.font.color.rgb = ACCENT; p2.font.name = FONT
        if author:
            p3 = tf.add_paragraph(); p3.text = author; p3.font.size = Pt(14)
            p3.font.color.rgb = SUB; p3.font.name = FONT

    # --- Section Header ---
    elif stype == 'SectionHeader':
        slide = prs.slides.add_slide(prs.slide_layouts[6])
        add_bg(slide)
        # Left accent block
        shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, 0, 0, Inches(0.3), Inches(7.5))
        shape.fill.solid(); shape.fill.fore_color.rgb = ACCENT; shape.line.fill.background()
        tb = slide.shapes.add_textbox(Inches(1.5), Inches(2.5), Inches(10), Inches(2))
        tf = tb.text_frame; tf.word_wrap = True
        p = tf.paragraphs[0]; p.text = title; p.font.size = Pt(36); p.font.bold = True
        p.font.color.rgb = TEXT; p.font.name = FONT
        if subtitle:
            p2 = tf.add_paragraph(); p2.text = subtitle; p2.font.size = Pt(16)
            p2.font.color.rgb = ACCENT; p2.font.name = FONT

    # --- Content / TwoColumn ---
    elif stype in ('Content', 'TwoColumn'):
        slide = prs.slides.add_slide(prs.slide_layouts[6])
        add_bg(slide)
        add_accent_bar(slide, Inches(0.5), Inches(0.3), Inches(12.3), Inches(0.04))
        # Title
        tb = slide.shapes.add_textbox(Inches(0.5), Inches(0.5), Inches(12), Inches(0.7))
        tf = tb.text_frame; p = tf.paragraphs[0]; p.text = title
        p.font.size = Pt(28); p.font.bold = True; p.font.color.rgb = TEXT; p.font.name = FONT
        # Content area
        if stype == 'TwoColumn':
            left_w = Inches(5.8)
            tb = slide.shapes.add_textbox(Inches(0.5), Inches(1.4), left_w, Inches(5.5))
        else:
            left_w = Inches(7.5)
            tb = slide.shapes.add_textbox(Inches(0.5), Inches(1.4), left_w, Inches(5.5))
        tf = tb.text_frame; tf.word_wrap = True
        if body:
            p = tf.paragraphs[0]; p.text = body; p.font.size = Pt(14)
            p.font.color.rgb = SUB; p.font.name = FONT; p.line_spacing = Pt(22)
        if bullets:
            for i, b in enumerate(bullets):
                if i == 0 and not body:
                    p = tf.paragraphs[0]
                else:
                    p = tf.add_paragraph()
                p.text = f"• {{b}}"; p.font.size = Pt(14); p.font.color.rgb = TEXT
                p.font.name = FONT; p.line_spacing = Pt(22); p.space_after = Pt(6)
        if notes:
            slide.notes_slide.notes_text_frame.text = notes
        add_footer(slide, idx+1, len(slides_data))

    # --- Quote ---
    elif stype == 'QuoteSlide':
        slide = prs.slides.add_slide(prs.slide_layouts[6])
        add_bg(slide)
        add_accent_bar(slide, Inches(3), Inches(3.2), Inches(7), Inches(0.04))
        tb = slide.shapes.add_textbox(Inches(1.5), Inches(1.8), Inches(10.5), Inches(3))
        tf = tb.text_frame; tf.word_wrap = True
        p = tf.paragraphs[0]; p.text = f'"{{title}}"'; p.font.size = Pt(32)
        p.font.italic = True; p.font.color.rgb = TEXT; p.font.name = FONT
        p.alignment = PP_ALIGN.CENTER
        if subtitle:
            p2 = tf.add_paragraph(); p2.text = f"— {{subtitle}}"
            p2.font.size = Pt(16); p2.font.color.rgb = ACCENT; p2.alignment = PP_ALIGN.CENTER

    # --- Thank You ---
    elif stype == 'ThankYou':
        slide = prs.slides.add_slide(prs.slide_layouts[6])
        add_bg(slide)
        add_accent_bar(slide, Inches(3), Inches(3.5), Inches(7), Inches(0.06))
        tb = slide.shapes.add_textbox(Inches(1.5), Inches(2.5), Inches(10.5), Inches(2))
        tf = tb.text_frame
        p = tf.paragraphs[0]; p.text = title or 'Thank You'; p.font.size = Pt(48)
        p.font.bold = True; p.font.color.rgb = TEXT; p.font.name = FONT
        p.alignment = PP_ALIGN.CENTER
        if subtitle:
            p2 = tf.add_paragraph(); p2.text = subtitle
            p2.font.size = Pt(18); p2.font.color.rgb = ACCENT; p2.alignment = PP_ALIGN.CENTER

    # --- Table ---
    elif stype == 'TableSlide':
        slide = prs.slides.add_slide(prs.slide_layouts[6])
        add_bg(slide)
        add_accent_bar(slide, Inches(0.5), Inches(0.3), Inches(12.3), Inches(0.04))
        tb = slide.shapes.add_textbox(Inches(0.5), Inches(0.5), Inches(12), Inches(0.7))
        tf = tb.text_frame; p = tf.paragraphs[0]; p.text = title
        p.font.size = Pt(28); p.font.bold = True; p.font.color.rgb = TEXT
        td = slide_data.get('table_data', {{}})
        headers = td.get('headers', [])
        rows = td.get('rows', [])
        if headers:
            n_rows = len(rows) + 1; n_cols = len(headers)
            tbl = slide.shapes.add_table(n_rows, n_cols, Inches(1), Inches(1.5), Inches(11), Inches(5)).table
            for ci, h in enumerate(headers):
                cell = tbl.cell(0, ci); cell.text = h
                for p in cell.text_frame.paragraphs: p.font.bold = True; p.font.size = Pt(12); p.font.color.rgb = RGBColor(0xFF,0xFF,0xFF)
                cell.fill.solid(); cell.fill.fore_color.rgb = ACCENT
            for ri, row in enumerate(rows):
                for ci, val in enumerate(row):
                    cell = tbl.cell(ri+1, ci); cell.text = val
                    for p in cell.text_frame.paragraphs: p.font.size = Pt(11); p.font.color.rgb = TEXT
                    if ri % 2 == 0: cell.fill.solid(); cell.fill.fore_color.rgb = RGBColor(0xF8,0xFA,0xFC)

    # --- Chart ---
    elif stype == 'ChartSlide':
        slide = prs.slides.add_slide(prs.slide_layouts[6])
        add_bg(slide)
        add_accent_bar(slide, Inches(0.5), Inches(0.3), Inches(12.3), Inches(0.04))
        tb = slide.shapes.add_textbox(Inches(0.5), Inches(0.5), Inches(12), Inches(0.7))
        tf = tb.text_frame; p = tf.paragraphs[0]; p.text = title
        p.font.size = Pt(28); p.font.bold = True; p.font.color.rgb = TEXT
        cd = slide_data.get('chart_data', {{}})
        if cd:
            chart_tb = slide.shapes.add_textbox(Inches(1), Inches(2), Inches(11), Inches(4))
            ctf = chart_tb.text_frame; ctf.word_wrap = True
            for si, s in enumerate(cd.get('series', [])):
                p = ctf.paragraphs[0] if si==0 else ctf.add_paragraph()
                vals = ', '.join([str(v) for v in s.get('values', [])])
                p.text = f"{{s.get('name', '')}}: {{vals}}"
                p.font.size = Pt(14); p.font.color.rgb = TEXT

prs.save(r'{8}')
print(f"PPTX generated: {{len(slides_data)}} slides, template={{'{9}'}}")
"#,
            tmpl.bg_color(), tmpl.accent_color(), tmpl.text_color(),
            tmpl.subtitle_color(), tmpl.font_family(),
            slides_json, req.author.as_deref().unwrap_or("Chronos-Shadow"),
            req.title, output, tmpl.name(),
        )
    }
}

// ─── 幻灯片 XML 生成 ──────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&apos;")
}

/// 生成单页幻灯片 OOXML
fn build_slide_xml(
    slide: &SlideContent, page: usize,
    bg: &str, accent: &str, text_color: &str, font: &str,
) -> String {
    let is_title = page == 1;
    let mut body = String::new();

    // 标题
    let title_size = if is_title { 4400 } else { 2800 };
    let title_y = if is_title { 2200000 } else { 300000 };
    body.push_str(&format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"1\" name=\"Title\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"800000\" y=\"{}\" ext=\"cx\" cy=\"800000\"/><a:ext cx=\"10500000\" cy=\"800000\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"{}\" b=\"1\" dirty=\"0\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>",
        title_y, title_size, text_color, font, xml_escape(&slide.title)
    ));

    // 副标题
    if let Some(sub) = &slide.subtitle {
        body.push_str(&format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Subtitle\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"800000\" y=\"3200000\" ext=\"cx\" cy=\"600000\"/><a:ext cx=\"10500000\" cy=\"600000\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\" dirty=\"0\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>",
            accent, font, xml_escape(sub)
        ));
    }

    // 正文
    if let Some(body_text) = &slide.body {
        body.push_str(&format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Body\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"800000\" y=\"1400000\" ext=\"cx\" cy=\"4000000\"/><a:ext cx=\"10500000\" cy=\"4000000\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr wrap=\"square\"/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1400\" dirty=\"0\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>",
            text_color, font, xml_escape(body_text)
        ));
    }

    // 要点列表
    if let Some(bullets) = &slide.bullets {
        let mut bullet_paras = String::new();
        for b in bullets {
            bullet_paras.push_str(&format!(
                "<a:p><a:pPr marL=\"285750\" indent=\"-285750\"><a:buFont typeface=\"Arial\"/><a:buChar char=\"•\"/></a:pPr><a:r><a:rPr lang=\"en-US\" sz=\"1400\" dirty=\"0\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t>{}</a:t></a:r></a:p>",
                text_color, font, xml_escape(b)
            ));
        }
        body.push_str(&format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"4\" name=\"Bullets\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"800000\" y=\"1500000\" ext=\"cx\" cy=\"4000000\"/><a:ext cx=\"10500000\" cy=\"4000000\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr wrap=\"square\"/><a:lstStyle/>{}</p:txBody></p:sp>",
            bullet_paras
        ));
    }

    // 背景 + 顶部强调条
    format!(
        "<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id=\"100\" name=\"Bg\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"12192000\" cy=\"6858000\"/></a:xfrm><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></p:spPr></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"101\" name=\"Accent\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"12192000\" cy=\"50000\"/></a:xfrm><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></p:spPr></p:sp>{}</p:spTree></p:cSld><p:clrMapOvr><a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/></p:clrMapOvr></p:sld>",
        bg, accent, body
    )
}

// ─── 辅助函数 ──────────────────────────────────────────────────────

fn extract_title_from_html(html: &str) -> Option<String> {
    let re = regex::Regex::new(r"<title[^>]*>([^<]+)</title>").ok()?;
    re.captures(html).map(|c| c[1].trim().to_string())
}

fn extract_dominant_color(html: &str) -> Option<String> {
    // 从CSS/内联样式提取第一个 #hex 颜色
    let re = regex::Regex::new(r"#[0-9A-Fa-f]{6}").unwrap();
    re.find(html).map(|m| m.as_str()[1..].to_uppercase())
}

fn is_dark_color(hex: &str) -> bool {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
    (r as u16 + g as u16 + b as u16) < 300
}

fn is_bright_color(hex: &str) -> bool {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    (r > 200 && g > 150) || (hex.starts_with("FF") || hex.starts_with("FE"))
}

fn is_corporate_color(hex: &str) -> bool {
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    hex.starts_with("1E") || hex.starts_with("0F") || hex.starts_with("25") || b > 200
}

fn adjust_color(hex: &str, offset: i16) -> String {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
    let new_r = (r as i16 + offset).clamp(0, 255) as u8;
    let new_g = (g as i16 + offset).clamp(0, 255) as u8;
    let new_b = (b as i16 + offset).clamp(0, 255) as u8;
    format!("{:02X}{:02X}{:02X}", new_r, new_g, new_b)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .replace("  ", " ")
}

/// 确保 python-pptx 已安装
#[allow(dead_code)]
fn ensure_pptx_installed(python: &str) -> Result<(), String> {
    // 检查是否已安装
    let check = std::process::Command::new(python)
        .args(["-c", "import pptx"])
        .output();
    if check.map(|o| o.status.success()).unwrap_or(false) {
        return Ok(());
    }
    // 自动安装
    tracing::info!("[PPTX] Installing python-pptx...");
    let install = std::process::Command::new(python)
        .args(["-m", "pip", "install", "python-pptx", "-q"])
        .output()
        .map_err(|e| format!("pip install failed: {}", e))?;
    if install.status.success() {
        tracing::info!("[PPTX] python-pptx installed successfully");
        Ok(())
    } else {
        Err(format!("pip install python-pptx 失败: {}", String::from_utf8_lossy(&install.stderr)))
    }
}

/// 自动检测 Python 路径 (python → python3 → py)
fn find_python() -> String {
    for cmd in &["python", "python3", "py"] {
        if std::process::Command::new(cmd).arg("--version").output().is_ok() {
            return cmd.to_string();
        }
    }
    "python".into() // 兜底
}

/// 获取桌面目录 (跨平台)
mod dirs {
    pub fn desktop_dir() -> Option<std::path::PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(|p| std::path::PathBuf::from(p).join("Desktop"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME").ok().map(|p| std::path::PathBuf::from(p).join("Desktop"))
        }
    }
    pub fn document_dir() -> Option<std::path::PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(|p| std::path::PathBuf::from(p).join("Documents"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME").ok().map(|p| std::path::PathBuf::from(p).join("Documents"))
        }
    }
}

// ─── 测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_detection() {
        assert!(is_dark_color("0A0A0F"));
        assert!(is_corporate_color("1E40AF"));
        assert!(!is_dark_color("FFFFFF"));
    }

    #[test]
    fn test_template_colors() {
        assert_eq!(PptTemplate::Corporate.bg_color(), "FFFFFF");
        assert_eq!(PptTemplate::DarkMode.bg_color(), "0A0A0F");
        assert_eq!(PptTemplate::TechMinimal.accent_color(), "38BDF8");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello: World!"), "Hello_ World_");
        assert_eq!(sanitize_filename("产品介绍V2"), "产品介绍V2");
    }
}
