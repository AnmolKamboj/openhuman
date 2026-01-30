import { describe, it, expect } from "vitest";
import * as tools from "../index";
import { TELEGRAM_MCP_TOOL_NAMES } from "../../types";
import type { MCPTool } from "../../../mcp/types";

describe("Tool Definitions", () => {
  // Extract tool definitions (exports ending with "Tool")
  const getToolDefinitions = (): Array<{ key: string; tool: MCPTool }> => {
    return Object.entries(tools)
      .filter(([key]) => key.endsWith("Tool"))
      .map(([key, tool]) => ({ key, tool: tool as MCPTool }));
  };

  // Extract handler functions (exports not ending with "Tool")
  const getHandlers = (): Array<{ key: string; handler: Function }> => {
    return Object.entries(tools)
      .filter(([key]) => !key.endsWith("Tool"))
      .filter(([, value]) => typeof value === "function")
      .map(([key, handler]) => ({ key, handler: handler as Function }));
  };

  it("should export tool definitions with name, description, and inputSchema", () => {
    const toolDefs = getToolDefinitions();

    expect(toolDefs.length).toBeGreaterThan(0);

    toolDefs.forEach(({ tool }) => {
      expect(tool).toBeDefined();
      expect(tool.name).toBeDefined();
      expect(typeof tool.name).toBe("string");
      expect(tool.name.length).toBeGreaterThan(0);

      expect(tool.description).toBeDefined();
      expect(typeof tool.description).toBe("string");
      expect(tool.description.length).toBeGreaterThan(0);

      expect(tool.inputSchema).toBeDefined();
      expect(tool.inputSchema.type).toBe("object");
    });
  });

  it("should export handler functions", () => {
    const handlers = getHandlers();

    expect(handlers.length).toBeGreaterThan(0);

    handlers.forEach(({ handler }) => {
      expect(handler).toBeDefined();
      expect(typeof handler).toBe("function");
    });
  });

  it("should have inputSchema with properties object", () => {
    const toolDefs = getToolDefinitions();

    toolDefs.forEach(({ tool }) => {
      expect(tool.inputSchema.properties).toBeDefined();
      expect(typeof tool.inputSchema.properties).toBe("object");
    });
  });

  it("should have all TELEGRAM_MCP_TOOL_NAMES with corresponding tool exports", () => {
    const exportedToolNames = getToolDefinitions().map(({ tool }) => tool.name);

    TELEGRAM_MCP_TOOL_NAMES.forEach((toolName) => {
      expect(
        exportedToolNames,
        `Tool "${toolName}" from TELEGRAM_MCP_TOOL_NAMES should be exported`
      ).toContain(toolName);
    });
  });

  it("should have unique tool names", () => {
    const toolNames = getToolDefinitions().map(({ tool }) => tool.name);
    const uniqueNames = new Set(toolNames);
    expect(uniqueNames.size).toBe(toolNames.length);
  });

  it("should have at least 79 tool definitions", () => {
    const toolCount = getToolDefinitions().length;
    expect(toolCount).toBeGreaterThanOrEqual(79);
  });

  it("should have required fields in inputSchema where appropriate", () => {
    const toolDefs = getToolDefinitions();

    toolDefs.forEach(({ tool }) => {
      if (tool.inputSchema.required) {
        expect(Array.isArray(tool.inputSchema.required)).toBe(true);

        tool.inputSchema.required.forEach((requiredField: string) => {
          expect(tool.inputSchema.properties).toHaveProperty(requiredField);
        });
      }
    });
  });

  it("should have valid property types in inputSchema", () => {
    const validTypes = [
      "string",
      "number",
      "integer",
      "boolean",
      "array",
      "object",
    ];
    const toolDefs = getToolDefinitions();

    toolDefs.forEach(({ tool }) => {
      Object.values(tool.inputSchema.properties).forEach((prop: any) => {
        if (prop.type) {
          expect(validTypes).toContain(prop.type);
        }
      });
    });
  });

  it("should have descriptions for all properties", () => {
    const toolDefs = getToolDefinitions();

    toolDefs.forEach(({ tool }) => {
      Object.entries(tool.inputSchema.properties).forEach(
        ([propName, prop]: [string, any]) => {
          expect(
            prop.description,
            `Property "${propName}" in tool "${tool.name}" should have a description`
          ).toBeDefined();
          expect(typeof prop.description).toBe("string");
          expect(prop.description.length).toBeGreaterThan(0);
        }
      );
    });
  });

  it("should match exported tool count with TELEGRAM_MCP_TOOL_NAMES count", () => {
    const exportedToolCount = getToolDefinitions().length;
    expect(exportedToolCount).toBe(TELEGRAM_MCP_TOOL_NAMES.length);
  });

  it("should have matching handler for each tool definition", () => {
    const toolDefs = getToolDefinitions();
    const handlers = getHandlers();

    expect(toolDefs.length).toBe(handlers.length);

    toolDefs.forEach(({ key: toolKey }) => {
      // Tool export key ends with "Tool", handler key should be the base name
      const baseName = toolKey.replace(/Tool$/, "");
      const hasMatchingHandler = handlers.some(({ key }) => key === baseName);
      expect(
        hasMatchingHandler,
        `Tool "${toolKey}" should have a matching handler "${baseName}"`
      ).toBe(true);
    });
  });
});
