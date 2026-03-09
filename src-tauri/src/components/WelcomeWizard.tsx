import { useState, useEffect, useRef } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
	initializeApp,
	InitRequest,
	InitResult,
	AppSettings,
	requestKeychainAccess,
	onInitProgress,
	InitProgress,
} from '../hooks/useArticles';
import './WelcomeWizard.css';

// Password input with visibility toggle
interface PasswordInputProps {
	value: string;
	onChange: (value: string) => void;
	placeholder?: string;
	inputRef?: React.RefObject<HTMLInputElement | null>;
	onKeyDown?: (e: React.KeyboardEvent<HTMLInputElement>) => void;
}

function PasswordInput({ value, onChange, placeholder, inputRef, onKeyDown }: PasswordInputProps) {
	const [showPassword, setShowPassword] = useState(false);

	return (
		<div className="password-input-wrapper">
			<input
				ref={inputRef}
				type={showPassword ? 'text' : 'password'}
				className="input-field password-field"
				placeholder={placeholder}
				value={value}
				onChange={(e) => onChange(e.target.value)}
				onKeyDown={onKeyDown}
			/>
			<button
				type="button"
				className="password-toggle-btn"
				onClick={() => setShowPassword(!showPassword)}
				title={showPassword ? '隐藏密码' : '显示密码'}
			>
				{showPassword ? (
					// Eye open icon (password is visible)
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
						<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
						<circle cx="12" cy="12" r="3"></circle>
					</svg>
				) : (
					// Eye closed icon (password is hidden)
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
						<path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path>
						<line x1="1" y1="1" x2="23" y2="23"></line>
					</svg>
				)}
			</button>
		</div>
	);
}

interface WelcomeWizardProps {
	onComplete: () => void;
}

type Step = 'categories' | 'papers' | 'embedding' | 'translation' | 'keychain' | 'params' | 'processing' | 'result';

const defaultSettings: AppSettings = {
	arxiv_categories: ['CS.AI', 'CS.LG', 'CS.CV'],
	siliconflow_api_key: '',
	embedding_api_base_url: 'https://api.siliconflow.cn/v1',
	embedding_api_key: '',
	embedding_model: 'BAAI/bge-m3',
	translation_api_base_url: 'https://api.openai.com/v1',
	translation_api_key: '',
	translation_model: 'gpt-3.5-turbo',
	pos_clusters: 5,
	neg_clusters: 3,
	daily_papers: 20,
	negative_alpha: 1.5,
	diversity_ratio: 0.3,
};

// 按学科分组的 arXiv 分类
interface CategoryGroup {
	name: string;
	categories: { code: string; name: string }[];
}

// @ts-ignore
const ARXIV_CATEGORY_GROUPS: CategoryGroup[] = [
	{
		name: 'Computer Science',
		categories: [
			{ code: 'CS.AI', name: 'Artificial Intelligence' },
			{ code: 'CS.AR', name: 'Hardware Architecture' },
			{ code: 'CS.CC', name: 'Computational Complexity' },
			{ code: 'CS.CE', name: 'Computational Engineering, Finance, and Science' },
			{ code: 'CS.CG', name: 'Computational Geometry' },
			{ code: 'CS.CL', name: 'Computation and Language' },
			{ code: 'CS.CR', name: 'Cryptography and Security' },
			{ code: 'CS.CV', name: 'Computer Vision and Pattern Recognition' },
			{ code: 'CS.CY', name: 'Computers and Society' },
			{ code: 'CS.DB', name: 'Databases' },
			{ code: 'CS.DC', name: 'Distributed, Parallel, and Cluster Computing' },
			{ code: 'CS.DL', name: 'Digital Libraries' },
			{ code: 'CS.DM', name: 'Discrete Mathematics' },
			{ code: 'CS.DS', name: 'Data Structures and Algorithms' },
			{ code: 'CS.ET', name: 'Emerging Technologies' },
			{ code: 'CS.FL', name: 'Formal Languages and Automata Theory' },
			{ code: 'CS.GL', name: 'General Literature' },
			{ code: 'CS.GR', name: 'Graphics' },
			{ code: 'CS.GT', name: 'Computer Science and Game Theory' },
			{ code: 'CS.HC', name: 'Human-Computer Interaction' },
			{ code: 'CS.IR', name: 'Information Retrieval' },
			{ code: 'CS.IT', name: 'Information Theory' },
			{ code: 'CS.LG', name: 'Machine Learning' },
			{ code: 'CS.LO', name: 'Logic in Computer Science' },
			{ code: 'CS.MA', name: 'Multiagent Systems' },
			{ code: 'CS.MM', name: 'Multimedia' },
			{ code: 'CS.MS', name: 'Mathematical Software' },
			{ code: 'CS.NA', name: 'Numerical Analysis' },
			{ code: 'CS.NE', name: 'Neural and Evolutionary Computing' },
			{ code: 'CS.NI', name: 'Networking and Internet Architecture' },
			{ code: 'CS.OH', name: 'Other Computer Science' },
			{ code: 'CS.OS', name: 'Operating Systems' },
			{ code: 'CS.PF', name: 'Performance' },
			{ code: 'CS.PL', name: 'Programming Languages' },
			{ code: 'CS.RO', name: 'Robotics' },
			{ code: 'CS.SC', name: 'Symbolic Computation' },
			{ code: 'CS.SD', name: 'Sound' },
			{ code: 'CS.SE', name: 'Software Engineering' },
			{ code: 'CS.SI', name: 'Social and Information Networks' },
			{ code: 'CS.SY', name: 'Systems and Control' },
		],
	},
	{
		name: 'Electrical Engineering and Systems Science',
		categories: [
			{ code: 'EESS.AS', name: 'Audio and Speech Processing' },
			{ code: 'EESS.IV', name: 'Image and Video Processing' },
			{ code: 'EESS.SP', name: 'Signal Processing' },
			{ code: 'EESS.SY', name: 'Systems and Control' },
		],
	},
	{
		name: 'Mathematics',
		categories: [
			{ code: 'MATH.AC', name: 'Commutative Algebra' },
			{ code: 'MATH.AG', name: 'Algebraic Geometry' },
			{ code: 'MATH.AP', name: 'Analysis of PDEs' },
			{ code: 'MATH.AT', name: 'Algebraic Topology' },
			{ code: 'MATH.CA', name: 'Classical Analysis and ODEs' },
			{ code: 'MATH.CO', name: 'Combinatorics' },
			{ code: 'MATH.CT', name: 'Category Theory' },
			{ code: 'MATH.CV', name: 'Complex Variables' },
			{ code: 'MATH.DG', name: 'Differential Geometry' },
			{ code: 'MATH.DS', name: 'Dynamical Systems' },
			{ code: 'MATH.FA', name: 'Functional Analysis' },
			{ code: 'MATH.GM', name: 'General Mathematics' },
			{ code: 'MATH.GN', name: 'General Topology' },
			{ code: 'MATH.GR', name: 'Group Theory' },
			{ code: 'MATH.GT', name: 'Geometric Topology' },
			{ code: 'MATH.HO', name: 'History and Overview' },
			{ code: 'MATH.IT', name: 'Information Theory' },
			{ code: 'MATH.KT', name: 'K-Theory and Homology' },
			{ code: 'MATH.LO', name: 'Logic' },
			{ code: 'MATH.MG', name: 'Metric Geometry' },
			{ code: 'MATH.MP', name: 'Mathematical Physics' },
			{ code: 'MATH.NA', name: 'Numerical Analysis' },
			{ code: 'MATH.NT', name: 'Number Theory' },
			{ code: 'MATH.OA', name: 'Operator Algebras' },
			{ code: 'MATH.OC', name: 'Optimization and Control' },
			{ code: 'MATH.PR', name: 'Probability' },
			{ code: 'MATH.QA', name: 'Quantum Algebra' },
			{ code: 'MATH.RA', name: 'Rings and Algebras' },
			{ code: 'MATH.RT', name: 'Representation Theory' },
			{ code: 'MATH.SG', name: 'Symplectic Geometry' },
			{ code: 'MATH.SP', name: 'Spectral Theory' },
			{ code: 'MATH.ST', name: 'Statistics Theory' },
		],
	},
	{
		name: 'Physics',
		categories: [
			{ code: 'ASTRO-PH.CO', name: 'Cosmology and Nongalactic Astrophysics' },
			{ code: 'ASTRO-PH.EP', name: 'Earth and Planetary Astrophysics' },
			{ code: 'ASTRO-PH.GA', name: 'Astrophysics of Galaxies' },
			{ code: 'ASTRO-PH.HE', name: 'High Energy Astrophysical Phenomena' },
			{ code: 'ASTRO-PH.IM', name: 'Instrumentation and Methods for Astrophysics' },
			{ code: 'ASTRO-PH.SR', name: 'Solar and Stellar Astrophysics' },
			{ code: 'COND-MAT.DIS-NN', name: 'Disordered Systems and Neural Networks' },
			{ code: 'COND-MAT.MES-HALL', name: 'Mesoscale and Nanoscale Physics' },
			{ code: 'COND-MAT.MTRL-SCI', name: 'Materials Science' },
			{ code: 'COND-MAT.OTHER', name: 'Other Condensed Matter' },
			{ code: 'COND-MAT.QUANT-GAS', name: 'Quantum Gases' },
			{ code: 'COND-MAT.SOFT', name: 'Soft Condensed Matter' },
			{ code: 'COND-MAT.STAT-MECH', name: 'Statistical Mechanics' },
			{ code: 'COND-MAT.STR-EL', name: 'Strongly Correlated Electrons' },
			{ code: 'COND-MAT.SUPR-CON', name: 'Superconductivity' },
			{ code: 'GR-QC', name: 'General Relativity and Quantum Cosmology' },
			{ code: 'HEP-EX', name: 'High Energy Physics - Experiment' },
			{ code: 'HEP-LAT', name: 'High Energy Physics - Lattice' },
			{ code: 'HEP-PH', name: 'High Energy Physics - Phenomenology' },
			{ code: 'HEP-TH', name: 'High Energy Physics - Theory' },
			{ code: 'MATH-PH', name: 'Mathematical Physics' },
			{ code: 'NLIN.AO', name: 'Adaptation and Self-Organizing Systems' },
			{ code: 'NLIN.CD', name: 'Chaotic Dynamics' },
			{ code: 'NLIN.CG', name: 'Cellular Automata and Lattice Gases' },
			{ code: 'NLIN.PS', name: 'Pattern Formation and Solitons' },
			{ code: 'NLIN.SI', name: 'Exactly Solvable and Integrable Systems' },
			{ code: 'NUCL-EX', name: 'Nuclear Experiment' },
			{ code: 'NUCL-TH', name: 'Nuclear Theory' },
			{ code: 'PHYSICS.ACC-PH', name: 'Accelerator Physics' },
			{ code: 'PHYSICS.AO-PH', name: 'Atmospheric and Oceanic Physics' },
			{ code: 'PHYSICS.APP-PH', name: 'Applied Physics' },
			{ code: 'PHYSICS.ATM-CLUS', name: 'Atomic and Molecular Clusters' },
			{ code: 'PHYSICS.ATOM-PH', name: 'Atomic Physics' },
			{ code: 'PHYSICS.BIO-PH', name: 'Biological Physics' },
			{ code: 'PHYSICS.CHEM-PH', name: 'Chemical Physics' },
			{ code: 'PHYSICS.CLASS-PH', name: 'Classical Physics' },
			{ code: 'PHYSICS.COMP-PH', name: 'Computational Physics' },
			{ code: 'PHYSICS.DATA-AN', name: 'Data Analysis, Statistics and Probability' },
			{ code: 'PHYSICS.ED-PH', name: 'Physics Education' },
			{ code: 'PHYSICS.FLU-DYN', name: 'Fluid Dynamics' },
			{ code: 'PHYSICS.GEN-PH', name: 'General Physics' },
			{ code: 'PHYSICS.GEO-PH', name: 'Geophysics' },
			{ code: 'PHYSICS.HIST-PH', name: 'History and Philosophy of Physics' },
			{ code: 'PHYSICS.INS-DET', name: 'Instrumentation and Detectors' },
			{ code: 'PHYSICS.MED-PH', name: 'Medical Physics' },
			{ code: 'PHYSICS.OPTICS', name: 'Optics' },
			{ code: 'PHYSICS.PLASM-PH', name: 'Plasma Physics' },
			{ code: 'PHYSICS.POP-PH', name: 'Popular Physics' },
			{ code: 'PHYSICS.SOC-PH', name: 'Physics and Society' },
			{ code: 'PHYSICS.SPACE-PH', name: 'Space Physics' },
			{ code: 'QUANT-PH', name: 'Quantum Physics' },
		],
	},
	{
		name: 'Quantitative Biology',
		categories: [
			{ code: 'Q-BIO.BM', name: 'Biomolecules' },
			{ code: 'Q-BIO.CB', name: 'Cell Behavior' },
			{ code: 'Q-BIO.GN', name: 'Genomics' },
			{ code: 'Q-BIO.MN', name: 'Molecular Networks' },
			{ code: 'Q-BIO.NC', name: 'Neurons and Cognition' },
			{ code: 'Q-BIO.OT', name: 'Other Quantitative Biology' },
			{ code: 'Q-BIO.PE', name: 'Populations and Evolution' },
			{ code: 'Q-BIO.QM', name: 'Quantitative Methods' },
			{ code: 'Q-BIO.SC', name: 'Subcellular Processes' },
			{ code: 'Q-BIO.TO', name: 'Tissues and Organs' },
		],
	},
	{
		name: 'Statistics',
		categories: [
			{ code: 'STAT.AP', name: 'Applications' },
			{ code: 'STAT.CO', name: 'Computation' },
			{ code: 'STAT.ME', name: 'Methodology' },
			{ code: 'STAT.ML', name: 'Machine Learning' },
			{ code: 'STAT.OT', name: 'Other Statistics' },
			{ code: 'STAT.TH', name: 'Statistics Theory' },
		],
	},
];

export function WelcomeWizard({ onComplete }: WelcomeWizardProps) {
	const [currentStep, setCurrentStep] = useState<Step>('categories');
	const [settings, setSettings] = useState<AppSettings>(defaultSettings);
	const [selectedCategories, setSelectedCategories] = useState<string[]>(['CS.AI', 'CS.LG', 'CS.CV']);
	const [categoryInput, setCategoryInput] = useState('');
	const [favoritePapers, setFavoritePapers] = useState('');
	const [result, setResult] = useState<InitResult | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [isProcessing, setIsProcessing] = useState(false);
	const [initProgress, setInitProgress] = useState<InitProgress | null>(null);
	// @ts-ignore
	const [expandedGroups, setExpandedGroups] = useState<string[]>(['Computer Science']);

	// Refs for focusing
	const papersInputRef = useRef<HTMLTextAreaElement>(null);
	const embeddingApiKeyInputRef = useRef<HTMLInputElement>(null);
	const translationApiKeyInputRef = useRef<HTMLInputElement>(null);

	// Validate arXiv ID or URL
	const validateArxivInput = (input: string): boolean => {
		const lines = input.split('\n').filter(l => l.trim());
		if (lines.length === 0) return true; // 空输入是允许的（跳过）

		const arxivIdPattern = /^(?:arxiv\.org\/(abs|pdf)\/)?(\d{4}\.\d{4,5}(v\d+)?|\d{4}\.\d{5})(?:\.pdf)?$/i;
		const urlPattern = /^https?:\/\/arxiv\.org\/(abs|pdf)\/(\d{4}\.\d{4,5}(v\d+)?)/i;

		for (const line of lines) {
			const trimmed = line.trim();
			if (!arxivIdPattern.test(trimmed) && !urlPattern.test(trimmed)) {
				return false;
			}
		}
		return true;
	};

	// Focus papers input when entering step
	useEffect(() => {
		if (currentStep === 'papers' && papersInputRef.current) {
			papersInputRef.current.focus();
		}
	}, [currentStep]);

	// Focus API key input when entering embedding step
	useEffect(() => {
		if (currentStep === 'embedding' && embeddingApiKeyInputRef.current) {
			embeddingApiKeyInputRef.current.focus();
		}
	}, [currentStep]);

	// Listen to init progress events
	useEffect(() => {
		let unlisten: (() => void) | undefined;

		const setupListener = async () => {
			if (currentStep === 'processing') {
				unlisten = await onInitProgress((progress) => {
					setInitProgress(progress);
				});
			}
		};

		setupListener();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	}, [currentStep]);

	// Category input handling
	const handleCategoryKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === 'Enter' || e.key === ',') {
			e.preventDefault();
			const value = categoryInput.trim().toUpperCase();
			if (value && !selectedCategories.includes(value)) {
				setSelectedCategories([...selectedCategories, value]);
				setCategoryInput('');
			} else if (!value) {
				// 空输入时直接进入下一页
				goNext();
			}
		} else if (e.key === 'Backspace' && !categoryInput && selectedCategories.length > 0) {
			setSelectedCategories(selectedCategories.slice(0, -1));
		}
	};

	const removeCategory = (cat: string) => {
		setSelectedCategories(selectedCategories.filter(c => c !== cat));
	};

	// @ts-ignore
	const addSuggestedCategory = (cat: string) => {
		const formatted = cat.toUpperCase();
		if (!selectedCategories.includes(formatted)) {
			setSelectedCategories([...selectedCategories, formatted]);
		}
	};

	// Navigation
	const canProceed = () => {
		switch (currentStep) {
			case 'categories':
				return selectedCategories.length > 0;
			case 'papers':
				// 空输入可以跳过，有输入时必须有效
				return favoritePapers.trim() === '' || validateArxivInput(favoritePapers);
			case 'embedding':
				return settings.embedding_api_key.length >= 10;
			case 'translation':
				return true;
			case 'keychain':
				return true; // 总是可以继续
			case 'params':
				return settings.pos_clusters > 0 && settings.neg_clusters > 0 && settings.daily_papers > 0;
			default:
				return true;
		}
	};

	const goNext = async () => {
		setError(null);
		switch (currentStep) {
			case 'categories':
				setCurrentStep('papers');
				break;
			case 'papers':
				// 验证输入
				if (favoritePapers.trim() && !validateArxivInput(favoritePapers)) {
					setError('请输入有效的 arXiv ID 或链接，例如：2501.12345 或 https://arxiv.org/abs/2501.12345');
					return;
				}
				setCurrentStep('embedding');
				break;
			case 'embedding':
				setCurrentStep('translation');
				break;
			case 'translation':
				setCurrentStep('keychain');
				break;
			case 'keychain':
				// 点击继续时请求钥匙串权限
				try {
					await requestKeychainAccess(settings.embedding_api_key);
				} catch (e) {
					console.warn('钥匙串访问可能失败:', e);
				}
				setCurrentStep('params');
				break;
			case 'params':
				handleInitialize();
				break;
			case 'result':
				onComplete();
				break;
		}
	};

	const goBack = () => {
		setError(null);
		switch (currentStep) {
			case 'papers':
				setCurrentStep('categories');
				break;
			case 'embedding':
				setCurrentStep('papers');
				break;
			case 'translation':
				setCurrentStep('embedding');
				break;
			case 'keychain':
				setCurrentStep('translation');
				break;
			case 'params':
				setCurrentStep('keychain');
				break;
		}
	};

	// Initialize app
	const handleInitialize = async () => {
		setIsProcessing(true);
		setCurrentStep('processing');
		setError(null);

		try {
			const request: InitRequest = {
				arxiv_categories: selectedCategories,
				favorite_papers: favoritePapers
					.split('\n')
					.map(l => l.trim())
					.filter(l => l.length > 0),
				siliconflow_api_key: settings.embedding_api_key,
				embedding_api_base_url: settings.embedding_api_base_url,
				embedding_api_key: settings.embedding_api_key,
				embedding_model: settings.embedding_model,
				translation_api_base_url: settings.translation_api_base_url,
				translation_api_key: settings.translation_api_key,
				translation_model: settings.translation_model,
				pos_clusters: settings.pos_clusters,
				neg_clusters: settings.neg_clusters,
				daily_papers: settings.daily_papers,
				negative_alpha: settings.negative_alpha,
				diversity_ratio: settings.diversity_ratio,
			};

			const initResult = await initializeApp(request);
			setResult(initResult);
			setCurrentStep('result');
		} catch (e) {
			setError(String(e));
			setCurrentStep('embedding');
		} finally {
			setIsProcessing(false);
		}
	};

	// Render steps
	const renderCategoriesStep = () => (
		<div className="step-content">
			<h2>选择感兴趣的 arXiv 分类</h2>
			<p className="description">
				ZenFlow 会从这些分类中获取最新论文。默认已选中 CS、ML、CV 热门领域。
			</p>

			<div className="form-group">
				<label>已选择的分类 ({selectedCategories.length})</label>
				<div className="category-input-container">
					{selectedCategories.map(cat => (
						<span key={cat} className="category-tag">
							{cat}
							<button onClick={() => removeCategory(cat)}>×</button>
						</span>
					))}
					<input
						type="text"
						className="category-input"
						placeholder={selectedCategories.length === 0 ? "输入分类代码，按回车添加" : ""}
						value={categoryInput}
						onChange={(e) => setCategoryInput(e.target.value)}
						onKeyDown={handleCategoryKeyDown}
					/>
				</div>
			</div>

			<div className="form-group">
				<p className="hint">
					输入 arXiv 分类代码（如 CS.AI、MATH.OC），按回车添加多个。
				</p>
			</div>
		</div>
	);

	const renderPapersStep = () => (
		<div className="step-content">
			<h2>添加喜欢的论文（可选）</h2>
			<p className="description">
				提供喜欢的论文链接或 arXiv ID，用于初始化推荐模型。
			</p>

			<div className="form-group">
				<label>论文链接或 arXiv ID</label>
				<textarea
					ref={papersInputRef}
					className="textarea-field"
					placeholder={`示例：
https://arxiv.org/abs/2501.12345
2501.12345
cs/9901001`}
					value={favoritePapers}
					onChange={(e) => setFavoritePapers(e.target.value)}
				/>
				<p className="hint">
					支持 arXiv 链接或直接输入 ID。可跳过此步骤。
				</p>
			</div>
		</div>
	);

	// 打开外部链接
	const handleOpenUrl = async (url: string) => {
		try {
			await openUrl(url);
		} catch (e) {
			console.error('Failed to open URL:', e);
		}
	};

	// 嵌入向量 API 配置页面
	const renderEmbeddingStep = () => {
		return (
			<div className="step-content">
				<h2>配置嵌入向量 API</h2>
				<p className="description">
					设置用于生成论文向量嵌入的 API（OpenAI 兼容格式）。推荐使用 SiliconFlow。
				</p>

				{error && <div className="error-message">{error}</div>}

				<div className="form-group">
					<label>API Key *</label>
					<PasswordInput
						inputRef={embeddingApiKeyInputRef}
						value={settings.embedding_api_key}
						onChange={(value) => setSettings({ ...settings, embedding_api_key: value })}
						placeholder="sk-xxxxxxxxxxxxxxxx"
						onKeyDown={(e) => {
							if (e.key === 'Enter' && settings.embedding_api_key.length >= 10) {
								goNext();
							}
						}}
					/>
					<p className="hint">
						从 <a href="#" onClick={(e) => { e.preventDefault(); handleOpenUrl('https://cloud.siliconflow.cn/'); }}>SiliconFlow</a> 或其他 OpenAI 兼容 API 获取
					</p>
				</div>

				<div className="form-group">
					<label>Base URL</label>
					<input
						type="text"
						className="input-field"
						placeholder="https://api.siliconflow.cn/v1"
						value={settings.embedding_api_base_url}
						onChange={(e) => setSettings({ ...settings, embedding_api_base_url: e.target.value })}
					/>
				</div>

				<div className="form-group">
					<label>Model</label>
					<input
						type="text"
						className="input-field"
						placeholder="BAAI/bge-m3"
						value={settings.embedding_model}
						onChange={(e) => setSettings({ ...settings, embedding_model: e.target.value })}
					/>
				</div>
			</div>
		);
	};

	// 翻译 API 配置页面
	const renderTranslationStep = () => {
		return (
			<div className="step-content compact">
				<h2>配置翻译 API（可选）</h2>
				<p className="description">
					设置用于翻译论文摘要的 LLM API（OpenAI 兼容格式）。不配置则不提供翻译功能。
				</p>

				{error && <div className="error-message">{error}</div>}

				<div className="form-group">
					<label>API Key</label>
					<PasswordInput
						inputRef={translationApiKeyInputRef}
						value={settings.translation_api_key}
						onChange={(value) => setSettings({ ...settings, translation_api_key: value })}
						placeholder="sk-xxxxxxxxxxxxxxxx"
						onKeyDown={(e) => {
							if (e.key === 'Enter') {
								goNext();
							}
						}}
					/>
					<p className="hint">
						推荐从 <a href="#" onClick={(e) => { e.preventDefault(); handleOpenUrl('https://www.modelscope.cn/'); }}>ModelScope</a> 获取免费 API Key
					</p>
				</div>

				<div className="form-group">
					<label>Base URL</label>
					<input
						type="text"
						className="input-field"
						placeholder="https://api.openai.com/v1"
						value={settings.translation_api_base_url}
						onChange={(e) => setSettings({ ...settings, translation_api_base_url: e.target.value })}
					/>
				</div>

				<div className="form-group">
					<label>Model</label>
					<input
						type="text"
						className="input-field"
						placeholder="gpt-3.5-turbo"
						value={settings.translation_model}
						onChange={(e) => setSettings({ ...settings, translation_model: e.target.value })}
					/>
				</div>
			</div>
		);
	};

	const renderKeychainStep = () => (
		<div className="step-content">
			<h2>安全存储设置</h2>
			<p className="description">
				ZenFlow 需要访问系统钥匙串来安全存储您的 API Key。
			</p>

			{error && <div className="error-message">{error}</div>}

			<div className="form-group">
				<div style={{ textAlign: 'center', margin: '2rem 0' }}>
					<div style={{ fontSize: '4rem', marginBottom: '1rem' }}>🔐</div>
					<p style={{ color: 'var(--text-primary)', marginBottom: '0.5rem', fontWeight: '500' }}>
						请点击"继续"按钮
					</p>
					<p className="hint" style={{ color: 'var(--accent)', marginBottom: '1rem' }}>
						系统将弹出密码对话框，请输入您的电脑密码<br />
						在对话框中勾选"始终允许"或"允许"选项
					</p>
					<p className="hint">
						这是 macOS 安全机制，用于安全存储您的 API Key<br />
						若不授权，API Key 将以明文形式存储在配置文件中
					</p>
				</div>
			</div>
		</div>
	);

	// Helper for number input validation
	// @ts-ignore
	const validateNumberInput = (value: string, min: number, max: number): number | null => {
		const num = parseInt(value, 10);
		if (isNaN(num) || num < min || num > max) return null;
		return num;
	};

	// Tooltip component
	const Tooltip = ({ children, text }: { children: React.ReactNode; text: string }) => (
		<span className="tooltip-container">
			{children}
			<span className="tooltip-text">{text}</span>
		</span>
	);

	const renderParamsStep = () => (
		<div className="step-content compact">
			<h2>配置推荐参数</h2>
			<p className="description">
				调整推荐算法参数（悬停标签查看说明）
			</p>

			<div className="form-group compact">
				<div className="param-grid">
					<div className="param-cell">
						<Tooltip text="值越大表示兴趣越广泛。建议：3-30">
							<label className="param-label">正向聚类数</label>
						</Tooltip>
						<input
							type="text"
							className="param-input no-spinner"
							value={settings.pos_clusters}
							onChange={(e) => {
								// 只允许最多2位数字
								const val = e.target.value.replace(/\D/g, '').slice(0, 2);
								const num = parseInt(val, 10);
								if (val === '' || (num >= 1 && num <= 100)) {
									setSettings({ ...settings, pos_clusters: num || 0 });
								}
							}}
						/>
					</div>

					<div className="param-cell">
						<Tooltip text="值越大可更精细过滤不感兴趣内容。建议：3-50">
							<label className="param-label">负向聚类数</label>
						</Tooltip>
						<input
							type="text"
							className="param-input no-spinner"
							value={settings.neg_clusters}
							onChange={(e) => {
								const val = e.target.value.replace(/\D/g, '').slice(0, 2);
								const num = parseInt(val, 10);
								if (val === '' || (num >= 1 && num <= 100)) {
									setSettings({ ...settings, neg_clusters: num || 0 });
								}
							}}
						/>
					</div>

					<div className="param-cell">
						<Tooltip text="每天展示的最大论文数量。建议：10-50">
							<label className="param-label">每日论文数</label>
						</Tooltip>
						<input
							type="text"
							className="param-input no-spinner"
							value={settings.daily_papers}
							onChange={(e) => {
								const val = e.target.value.replace(/\D/g, '').slice(0, 2);
								const num = parseInt(val, 10);
								if (val === '' || (num >= 1 && num <= 100)) {
									setSettings({ ...settings, daily_papers: num || 0 });
								}
							}}
						/>
					</div>
				</div>
			</div>

			<div className="form-group compact">
				<div className="param-row">
					<Tooltip text="α 越大，对不喜欢的内容越敏感。α=1.5 表示不喜欢的内容惩罚加重50%">
						<label className="param-label">负向惩罚系数 (α)</label>
					</Tooltip>
					<span className="param-value">{settings.negative_alpha.toFixed(1)}</span>
				</div>
				<input
					type="range"
					className="slider"
					min="0.5"
					max="3.0"
					step="0.1"
					value={settings.negative_alpha}
					onChange={(e) => setSettings({ ...settings, negative_alpha: parseFloat(e.target.value) })}
				/>
			</div>

			<div className="form-group compact">
				<div className="param-row">
					<Tooltip text="随机探索比例。30% 表示每10篇论文中有3篇是随机推荐的，帮助发现新领域">
						<label className="param-label">多样性比例</label>
					</Tooltip>
					<span className="param-value">{Math.round(settings.diversity_ratio * 100)}%</span>
				</div>
				<input
					type="range"
					className="slider"
					min="0"
					max="0.5"
					step="0.05"
					value={settings.diversity_ratio}
					onChange={(e) => setSettings({ ...settings, diversity_ratio: parseFloat(e.target.value) })}
				/>
				<div className="slider-labels">
					<span>纯推荐</span>
					<span>混合</span>
					<span>多探索</span>
				</div>
			</div>
		</div>
	);

	const renderProcessingStep = () => {
		const progress = initProgress?.progress ?? 0;
		const message = initProgress?.message ?? '正在初始化...';
		const detail = initProgress?.detail;

		return (
			<div className="step-content" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', minHeight: '300px', padding: '2rem' }}>
				<div style={{ width: '100%', maxWidth: '400px' }}>
					<div className="progress-bar-container" style={{ marginBottom: '1.5rem' }}>
						<div
							className="progress-bar"
							style={{
								width: `${Math.round(progress * 100)}%`,
								transition: 'width 0.3s ease'
							}}
						/>
					</div>
					<p className="loading-text" style={{ textAlign: 'center', marginBottom: '0.5rem' }}>
						{message}
					</p>
					{promptMsg && (
						<p className="hint" style={{ textAlign: 'center', marginBottom: '0.5rem', color: 'var(--accent)' }}>
							{promptMsg}
						</p>
					)}
					{detail && (
						<p className="hint" style={{ textAlign: 'center' }}>
							{detail}
						</p>
					)}
					<p className="hint" style={{ textAlign: 'center', marginTop: '1rem' }}>
						{Math.round(progress * 100)}% 完成
					</p>
				</div>
			</div>
		);
	};

	// Map stage to display message
	const stageMessages: Record<string, string> = {
		'clear': '清空数据库',
		'save_settings': '保存设置',
		'fetch_favorites': '获取偏好论文',
		'embed_favorites': '生成偏好论文向量',
		'clustering': '执行聚类分析',
		'fetch_rss': '抓取今日论文',
		'embed_rss': '生成论文向量',
		'complete': '初始化完成',
	};

	const promptMsg = initProgress ? stageMessages[initProgress.stage] : undefined;

	const renderResultStep = () => {
		if (!result) return null;

		return (
			<div className="step-content">
				<div className="result-container">
					<div className="result-icon">{result.errors.length === 0 ? '🎉' : '⚠️'}</div>
					<h2 className="result-title">{result.errors.length === 0 ? '初始化完成！' : '初始化完成（有警告）'}</h2>
					<p className="description">
						{result.errors.length === 0
							? 'ZenFlow 已准备就绪，开始探索学术论文吧！'
							: 'ZenFlow 已初始化，但过程中遇到了一些问题。'}
					</p>

					<div className="result-stats">
						<div className="result-stat">
							<div className="result-stat-value">{result.papers_fetched}</div>
							<div className="result-stat-label">获取论文</div>
						</div>
						<div className="result-stat">
							<div className="result-stat-value">{result.papers_embedded}</div>
							<div className="result-stat-label">生成向量</div>
						</div>
						<div className="result-stat">
							<div className="result-stat-value">{result.pos_clusters}</div>
							<div className="result-stat-label">正向偏好</div>
						</div>
					</div>

					{result.errors.length > 0 && (
						<div style={{ marginTop: '1rem', textAlign: 'left' }}>
							<p style={{ color: 'var(--danger)', fontSize: '0.875rem', marginBottom: '0.5rem' }}>警告/错误：</p>
							<ul style={{ fontSize: '0.8rem', color: 'var(--text-muted)', paddingLeft: '1.5rem' }}>
								{result.errors.map((err, i) => (
									<li key={i}>{err}</li>
								))}
							</ul>
						</div>
					)}
				</div>
			</div>
		);
	};

	const steps: { key: Step; label: string }[] = [
		{ key: 'categories', label: '选择分类' },
		{ key: 'papers', label: '添加论文' },
		{ key: 'embedding', label: '嵌入向量' },
		{ key: 'translation', label: '翻译API' },
		{ key: 'keychain', label: '安全存储' },
		{ key: 'params', label: '参数设置' },
		{ key: 'result', label: '完成' },
	];

	const getStepStatus = (stepKey: Step): 'pending' | 'active' | 'completed' => {
		const stepOrder: Step[] = ['categories', 'papers', 'embedding', 'translation', 'keychain', 'params', 'processing', 'result'];
		const currentIdx = stepOrder.indexOf(currentStep);
		const stepIdx = stepOrder.indexOf(stepKey);

		if (stepIdx < currentIdx) return 'completed';
		if (stepIdx === currentIdx) return 'active';
		return 'pending';
	};

	return (
		<div className="welcome-overlay">
			<div className="welcome-container">
				<div className="welcome-header">
					<h1>🚀 ZenFlow</h1>
					<p className="subtitle">AI 驱动的学术论文推荐系统</p>
				</div>

				<div className="progress-steps">
					{steps.map((step) => {
						const status = getStepStatus(step.key);
						return (
							<div key={step.key} className={`step ${status}`}>
								<span className="step-number">
									{status === 'completed' ? '✓' : steps.indexOf(step) + 1}
								</span>
								<span>{step.label}</span>
							</div>
						);
					})}
				</div>

				<div className="welcome-content">
					{currentStep === 'categories' && renderCategoriesStep()}
					{currentStep === 'papers' && renderPapersStep()}
					{currentStep === 'embedding' && renderEmbeddingStep()}
					{currentStep === 'translation' && renderTranslationStep()}
					{currentStep === 'keychain' && renderKeychainStep()}
					{currentStep === 'params' && renderParamsStep()}
					{currentStep === 'processing' && renderProcessingStep()}
					{currentStep === 'result' && renderResultStep()}
				</div>

				{currentStep !== 'processing' && (
					<div className="welcome-footer">
						{currentStep !== 'categories' && currentStep !== 'result' ? (
							<button className="btn-secondary" onClick={goBack}>
								← 上一步
							</button>
						) : (
							<div></div>
						)}

						<button
							className="btn-primary"
							onClick={goNext}
							disabled={!canProceed() || isProcessing}
						>
							{currentStep === 'result' ? '开始使用 →' : currentStep === 'papers' && !favoritePapers.trim() ? '跳过 →' : currentStep === 'translation' && settings.translation_api_key.length < 10 ? '跳过（不翻译）→' : '下一步 →'}
						</button>
					</div>
				)}
			</div>
		</div>
	);
}
