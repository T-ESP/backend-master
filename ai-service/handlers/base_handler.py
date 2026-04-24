from abc import ABC, abstractmethod
from utils.logger import get_logger


class BaseHandler(ABC):
    def __init__(self):
        self.logger = get_logger(self.__class__.__name__)

    @abstractmethod
    def run(self):
        pass
