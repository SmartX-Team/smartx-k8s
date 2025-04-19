#!/usr/bin/env python

from abc import ABCMeta, abstractmethod
import logging
from pathlib import Path
from typing import Iterable, Literal, final, override

from kafka import KafkaConsumer, KafkaProducer
from ollama import Client as OllamaClient, Message as OllamaMessage
from openai import Client as OpenAIClient
from openai.types import chat as openai_chat
from pydantic import BaseModel, SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict
import yaml

# Load environment variables from `.env` file
try:
    from dotenv import load_dotenv
    load_dotenv()
except ImportError:
    pass


class AgentBase[S: BaseSettings, I, O](metaclass=ABCMeta):
    '''
    A virtual agent base class.
    '''

    def __init__(self) -> None:
        self._logger = logging.getLogger(__name__)
        logging.basicConfig(
            encoding='utf-8',
            level=logging.INFO,
        )
        self._logger.info(f'Welcome to the {self.name}!')
        self._settings = self._settings_type()()

    @classmethod
    @abstractmethod
    def _settings_type(cls) -> type[S]:
        '''
        Return the settings class type.
        '''
        pass

    @property
    @abstractmethod
    def name(self) -> str:
        '''
        Return the agent name.
        '''
        pass

    @final
    @property
    def settings(self) -> S:
        '''
        Return the settings.
        '''
        return self._settings

    @final
    def exec(self) -> None:
        '''
        Consume the process and execute the agent task.
        '''
        self._logger.info('Started agent')
        try:
            while True:
                self._produce(self._process(self._consume()))
        except KeyboardInterrupt:
            self._logger.info('CTRL+C captured')
            return self.terminate()

    def _consume(self) -> I | None:
        '''
        Read a data.
        '''
        return None

    def _process(self, data: I | None) -> O | None:
        '''
        Process the data.
        '''
        return data

    def _produce(self, data: O) -> None:
        '''
        Write the data.
        '''
        pass

    def terminate(self) -> None:
        '''
        Terminate the agent.
        '''
        pass


class AgentSettings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file='.env',
        env_file_encoding='utf-8',
        extra='ignore',
    )

    kafka_bootstrap_servers: str
    kafka_group_id: str
    kafka_topic_src: str | None = None
    kafka_topic_sink: str | None = None

    ollama_base_url: str | None = None
    ollama_model_name: str | None = None

    openai_api_key: SecretStr | None = None
    openai_base_url: str = 'https://api.openai.com'
    openai_model_name: str | None = None

    prompt_home: Path = Path('.')

    def kafka_bootstrap_servers_list(self) -> list[str]:
        return self.kafka_bootstrap_servers.split(',')

    def kafka_topic_src_list(self) -> list[str]:
        return self.kafka_topic_src.split(',') \
            if self.kafka_topic_src \
            else []

    def kafka_topic_sink_list(self) -> list[str]:
        return self.kafka_topic_sink.split(',') \
            if self.kafka_topic_sink \
            else []


class Message(BaseModel):
    content: str


class PromptTemplate(BaseModel):
    role: Literal['system', 'user']
    '''The role of the messages author.'''

    name: str | None = None
    '''The optional name for the participant.'''

    template: str
    '''The content template of the message.'''

    def build_ollama(self) -> OllamaMessage:
        return OllamaMessage(
            role=self.role,
            content=self.template,
        )

    def build_openai(self) -> openai_chat.ChatCompletionMessageParam:
        match self.role:
            case 'system':
                return openai_chat.ChatCompletionSystemMessageParam(
                    role='system',
                    name=self.name or '',
                    content=self.template,
                )
            case 'user':
                return openai_chat.ChatCompletionUserMessageParam(
                    role='user',
                    name=self.name or '',
                    content=self.template,
                )


class Agent(AgentBase[AgentSettings, Message, Message]):
    def __init__(self) -> None:
        super().__init__()

        bootstrapper_servers = self.settings.kafka_bootstrap_servers_list()
        topic_src_list = self.settings.kafka_topic_src_list()
        topic_sink_list = self.settings.kafka_topic_sink_list()

        if topic_src_list:
            self._consumer = KafkaConsumer(
                bootstrap_servers=bootstrapper_servers,
                group_id=self.settings.kafka_group_id,
            )
            self._consumer.subscribe(topic_src_list)
            assert self._consumer.bootstrap_connected()
        else:
            self._consumer = None

        if topic_sink_list:
            self._producer = KafkaProducer(
                bootstrap_servers=bootstrapper_servers,
            )
        else:
            self._producer = None

        if self.settings.ollama_model_name is not None:
            self._llm = OllamaClient(
                host=self.settings.ollama_base_url,
            )
        elif self.settings.openai_model_name is not None:
            self._llm = OpenAIClient(
                api_key=self.settings.openai_api_key.get_secret_value(),
                base_url=self.settings.openai_base_url,
            )
        else:
            self._llm = None

        if self._llm is not None:
            with open(
                file=self.settings.prompt_home.joinpath('prompt.yaml'),
                encoding='utf-8',
            ) as fp:
                prompt_data = yaml.safe_load(fp)
                if not isinstance(prompt_data, list):
                    raise ValueError('Prompt is not a list')
                if not prompt_data:
                    raise ValueError('Empty prompt')
                self._prompt = [
                    PromptTemplate.model_validate(p)
                    for p in prompt_data
                ]
        else:
            self._prompt = []

        self._topic_sink = topic_sink_list

    @property
    @final
    @override
    def name(self) -> str:
        return 'H2PC LLM Agent'

    @classmethod
    @final
    @override
    def _settings_type(cls) -> type[AgentSettings]:
        return AgentSettings

    @final
    @override
    def _consume(self) -> Message | None:
        if self._consumer is not None:
            return Message.model_validate_json(next(self._consumer).value)
        else:
            return None

    @final
    def _build_ollama_prompt(
        self,
    ) -> Iterable[OllamaMessage]:
        return (
            message.build_ollama()
            for message in self._prompt
        )

    @final
    def _build_openai_prompt(
        self,
    ) -> Iterable[openai_chat.ChatCompletionMessageParam]:
        return (
            message.build_openai()
            for message in self._prompt
        )

    @final
    @override
    def _process(self, data: Message | None) -> Message | None:
        if isinstance(self._llm, OllamaClient):
            completion = self._llm.chat(
                model=self.settings.ollama_model_name,
                messages=self._build_ollama_prompt(),
            )
            content = completion.message.content
        elif isinstance(self._llm, OpenAIClient):
            completion = self._llm.chat.completions.create(
                model=self.settings.openai_model_name,
                messages=self._build_openai_prompt(),
            )
            content = completion.choices[0].message.content
        else:
            return super()._process(data)

        if content is not None:
            return Message(
                content=content,
            )
        else:
            return None

    @final
    @override
    def _produce(self, data: Message | None) -> None:
        if data is not None and self._producer is not None:
            value = data.model_dump_json().encode('utf-8')
            for topic in self._topic_sink:
                return self._producer.send(topic, value)

    @final
    @override
    def terminate(self) -> None:
        if self._consumer is not None:
            self._consumer.close()
        if self._producer is not None:
            self._producer.close()


if __name__ == '__main__':
    Agent().exec()
