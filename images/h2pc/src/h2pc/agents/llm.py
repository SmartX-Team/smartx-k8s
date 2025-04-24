from pathlib import Path
from typing import final, override

from jinja2 import Template
from pydantic import SecretStr
import yaml

from ..agents.base import AgentBase, AgentBaseSettings
from ..llms.base import BaseLLM
from ..llms.ollama import OllamaLLM
from ..llms.openai import OpenAILLM
from ..models.message import Message
from ..models.prompt import PromptTemplate


class _PromptTemplateRenderer:
    def __init__(self, path: Path) -> None:
        with open(
            file=path,
            encoding='utf-8',
        ) as fp:
            self._template = Template(fp.read())

    @final
    def render(
        self,
        data: list[Message],
    ) -> PromptTemplate:
        return PromptTemplate.model_validate(yaml.safe_load(
            stream=self._template.render(
                inputs=data,
            )
        ))


class LLMAgentSettings(AgentBaseSettings):
    ollama_base_url: str | None = None
    ollama_model_name: str | None = None

    openai_api_key: SecretStr | None = None
    openai_base_url: str = 'https://api.openai.com'
    openai_model_name: str | None = None


class LLMAgent(AgentBase[LLMAgentSettings, Message, Message]):
    def __init__(self) -> None:
        super().__init__()

        self._llm: BaseLLM
        if self.settings.ollama_base_url is not None and \
                self.settings.ollama_model_name is not None:
            self._llm = OllamaLLM(
                base_url=self.settings.ollama_base_url,
                model_name=self.settings.ollama_model_name,
            )
        elif self.settings.openai_api_key is not None and \
                self.settings.openai_model_name is not None:
            self._llm = OpenAILLM(
                api_key=self.settings.openai_api_key,
                base_url=self.settings.openai_base_url,
                model_name=self.settings.openai_model_name,
            )
        else:
            raise ValueError('No LLMs are available')

        self._template = _PromptTemplateRenderer(self.settings.prompt_path)

    @property
    @final
    @override
    def name(self) -> str:
        return 'LLM Agent'

    @property
    @final
    @override
    def _settings_type(cls) -> type[LLMAgentSettings]:
        return LLMAgentSettings

    @property
    @final
    @override
    def _input_type(cls) -> type[Message]:
        '''
        Return the input data class type.
        '''
        return Message

    @final
    @override
    async def process(self, data: list[Message]) -> Message | None:
        template = self._template.render(data)
        inputs = template.inputs

        # Generate content
        message = await self._llm.completions(inputs)

        # Validate output content
        if message is None:
            return None
        if self.settings.verbose:
            self._logger.info(f'Generated: {message.content}')

        # Filter content
        content = message.content.strip()
        if content.startswith('<thought>'):
            content = content.split('</thought>')[-1].strip()

        # Parse content
        match template.operator:
            case 'Index':
                index = int(content)
                if index < 1 or index >= len(data):
                    return None
                return data[index - 1]
            case 'Message':
                return Message(
                    role='assistant',
                    content=content,
                )
