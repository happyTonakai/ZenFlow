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
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
						<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
						<circle cx="12" cy="12" r="3"></circle>
					</svg>
				) : (
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

type Step = 'categories' | 'papers' | 'scoring' | 'translation' | 'keychain' | 'params' | 'processing' | 'result';

const defaultSettings: AppSettings = {
	arxiv_categories: ['CS.AI', 'CS.LG', 'CS.CV'],
	scoring_api_base_url: 'https://api.openai.com/v1',
	scoring_api_key: '',
	scoring_model: 'gpt-4o-mini',
	translation_api_base_url: 'https://api.openai.com/v1',
	translation_api_key: '',
	translation_model: 'gpt-3.5-turbo',
	daily_papers: 20,
	diversity_ratio: 0.3,
};

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

	// Refs for focusing
	const papersInputRef = useRef<HTMLTextAreaElement>(null);
	const scoringApiKeyInputRef = useRef<HTMLInputElement>(null);
	const translationApiKeyInputRef = useRef<HTMLInputElement>(null);

	// Validate arXiv ID or URL
	const validateArxivInput = (input: string): boolean => {
		const lines = input.split('\n').filter(l => l.trim());
		if (lines.length === 0) return true;

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

	// Focus API key input when entering scoring step
	useEffect(() => {
		if (currentStep === 'scoring' && scoringApiKeyInputRef.current) {
			scoringApiKeyInputRef.current.focus();
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
				goNext();
			}
		} else if (e.key === 'Backspace' && !categoryInput && selectedCategories.length > 0) {
			setSelectedCategories(selectedCategories.slice(0, -1));
		}
	};

	const removeCategory = (cat: string) => {
		setSelectedCategories(selectedCategories.filter(c => c !== cat));
	};

	// Navigation
	const canProceed = () => {
		switch (currentStep) {
			case 'categories':
				return selectedCategories.length > 0;
			case 'papers':
				return favoritePapers.trim() === '' || validateArxivInput(favoritePapers);
			case 'scoring':
				return settings.scoring_api_key.length >= 10;
			case 'translation':
				return true;
			case 'keychain':
				return true;
			case 'params':
				return settings.daily_papers > 0;
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
				if (favoritePapers.trim() && !validateArxivInput(favoritePapers)) {
					setError('请输入有效的 arXiv ID 或链接');
					return;
				}
				setCurrentStep('scoring');
				break;
			case 'scoring':
				setCurrentStep('translation');
				break;
			case 'translation':
				setCurrentStep('keychain');
				break;
			case 'keychain':
				try {
					await requestKeychainAccess(settings.scoring_api_key);
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
			case 'scoring':
				setCurrentStep('papers');
				break;
			case 'translation':
				setCurrentStep('scoring');
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
				scoring_api_base_url: settings.scoring_api_base_url,
				scoring_api_key: settings.scoring_api_key,
				scoring_model: settings.scoring_model,
				translation_api_base_url: settings.translation_api_base_url,
				translation_api_key: settings.translation_api_key,
				translation_model: settings.translation_model,
				daily_papers: settings.daily_papers,
				diversity_ratio: settings.diversity_ratio,
			};

			const initResult = await initializeApp(request);
			setResult(initResult);
			setCurrentStep('result');
		} catch (e) {
			setError(String(e));
			setCurrentStep('scoring');
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
							<button onClick={() => removeCategory(cat)}>x</button>
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
2501.12345`}
					value={favoritePapers}
					onChange={(e) => setFavoritePapers(e.target.value)}
				/>
				<p className="hint">
					支持 arXiv 链接或直接输入 ID。可跳过此步骤。
				</p>
			</div>
		</div>
	);

	const handleOpenUrl = async (url: string) => {
		try {
			await openUrl(url);
		} catch (e) {
			console.error('Failed to open URL:', e);
		}
	};

	// 评分 API 配置页面
	const renderScoringStep = () => {
		return (
			<div className="step-content">
				<h2>配置推荐 API</h2>
				<p className="description">
					设置用于论文评分和偏好分析的 LLM API（OpenAI 兼容格式）。
				</p>

				{error && <div className="error-message">{error}</div>}

				<div className="form-group">
					<label>API Key *</label>
					<PasswordInput
						inputRef={scoringApiKeyInputRef}
						value={settings.scoring_api_key}
						onChange={(value) => setSettings({ ...settings, scoring_api_key: value })}
						placeholder="sk-xxxxxxxxxxxxxxxx"
						onKeyDown={(e) => {
							if (e.key === 'Enter' && settings.scoring_api_key.length >= 10) {
								goNext();
							}
						}}
					/>
					<p className="hint">
						从 <a href="#" onClick={(e) => { e.preventDefault(); handleOpenUrl('https://platform.openai.com/'); }}>OpenAI</a> 或其他 OpenAI 兼容 API 获取
					</p>
				</div>

				<div className="form-group">
					<label>Base URL</label>
					<input
						type="text"
						className="input-field"
						placeholder="https://api.openai.com/v1"
						value={settings.scoring_api_base_url}
						onChange={(e) => setSettings({ ...settings, scoring_api_base_url: e.target.value })}
					/>
				</div>

				<div className="form-group">
					<label>Model</label>
					<input
						type="text"
						className="input-field"
						placeholder="gpt-4o-mini"
						value={settings.scoring_model}
						onChange={(e) => setSettings({ ...settings, scoring_model: e.target.value })}
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
						留空则使用与推荐 API 相同的配置，或从其他服务获取
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
		'generate_preferences': '分析用户偏好',
		'fetch_rss': '抓取今日论文',
		'scoring': '为论文评分',
		'translate': '翻译推荐论文',
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
							<div className="result-stat-value">{result.articles_scored}</div>
							<div className="result-stat-label">已评分</div>
						</div>
						<div className="result-stat">
							<div className="result-stat-value">{result.preferences_generated ? '✓' : '✗'}</div>
							<div className="result-stat-label">偏好生成</div>
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
		{ key: 'scoring', label: '推荐API' },
		{ key: 'translation', label: '翻译API' },
		{ key: 'keychain', label: '安全存储' },
		{ key: 'params', label: '参数设置' },
		{ key: 'result', label: '完成' },
	];

	const getStepStatus = (stepKey: Step): 'pending' | 'active' | 'completed' => {
		const stepOrder: Step[] = ['categories', 'papers', 'scoring', 'translation', 'keychain', 'params', 'processing', 'result'];
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
					{currentStep === 'scoring' && renderScoringStep()}
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
