using Blazor.Diagrams;
using Blazor.Diagrams.Core.Geometry;
using Blazor.Diagrams.Core.Models;
using SemanticGraph.Visualizer.Client.Models;

namespace SemanticGraph.Visualizer.Client.Services;

public static class GraphDiagramBuilder
{
    private static readonly Size FileNodeSize = new(220, 56);
    private static readonly Size SymbolNodeSize = new(220, 56);

    public static void Populate(BlazorDiagram diagram, GraphProjectionDto projection)
    {
        diagram.Links.Clear();
        diagram.Nodes.Clear();

        var diagramNodes = BuildNodes(projection.Nodes);
        diagram.Nodes.Add(diagramNodes.Values);

        var links = BuildLinks(projection.Edges, diagramNodes);
        diagram.Links.Add(links);
    }

    private static Dictionary<string, SemanticGraphNodeModel> BuildNodes(
        IReadOnlyList<GraphNodeDto> nodes)
    {
        var orderedNodes = nodes
            .OrderBy(node => node.SourceFilePath ?? string.Empty)
            .ThenBy(node => node.Kind)
            .ThenBy(node => node.QualifiedName ?? node.DisplayLabel)
            .ThenBy(node => node.Id)
            .ToArray();

        var fileNodes = orderedNodes
            .Where(node => string.Equals(node.Kind, "file", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        var rowByPath = fileNodes
            .Select((node, index) => new { Path = node.SourceFilePath ?? node.QualifiedName ?? node.Id, Index = index })
            .ToDictionary(item => item.Path, item => item.Index, StringComparer.Ordinal);

        var symbolIndexByPath = new Dictionary<string, int>(StringComparer.Ordinal);
        var result = new Dictionary<string, SemanticGraphNodeModel>(StringComparer.Ordinal);

        foreach (var node in orderedNodes)
        {
            var view = GraphNodeViewModel.FromDto(node);
            var path = node.SourceFilePath ?? node.QualifiedName ?? node.Id;
            var position = view.IsFile
                ? FilePosition(rowByPath.GetValueOrDefault(path, rowByPath.Count))
                : SymbolPosition(path, rowByPath, symbolIndexByPath);

            result[node.Id] = new SemanticGraphNodeModel(
                view,
                position,
                view.IsFile ? FileNodeSize : SymbolNodeSize);
        }

        return result;
    }

    private static IReadOnlyList<SemanticGraphLinkModel> BuildLinks(
        IReadOnlyList<GraphEdgeDto> edges,
        IReadOnlyDictionary<string, SemanticGraphNodeModel> nodes)
    {
        var links = new List<SemanticGraphLinkModel>();

        foreach (var edge in edges.OrderBy(edge => edge.Relation).ThenBy(edge => edge.Id))
        {
            if (!nodes.TryGetValue(edge.SourceNodeId, out var source)
                || !nodes.TryGetValue(edge.TargetNodeId, out var target))
            {
                continue;
            }

            var link = new SemanticGraphLinkModel(edge, source, target);
            ApplyRelationStyle(link, edge.Relation);
            links.Add(link);
        }

        return links;
    }

    private static Point FilePosition(int row)
    {
        return new Point(48, 56 + row * 220);
    }

    private static Point SymbolPosition(
        string path,
        IReadOnlyDictionary<string, int> rowByPath,
        IDictionary<string, int> symbolIndexByPath)
    {
        var row = rowByPath.GetValueOrDefault(path, rowByPath.Count);
        var symbolIndex = symbolIndexByPath.TryGetValue(path, out var currentSymbolIndex)
            ? currentSymbolIndex
            : 0;
        symbolIndexByPath[path] = symbolIndex + 1;

        var column = symbolIndex / 5;
        var slot = symbolIndex % 5;

        return new Point(330 + column * 260, 36 + row * 220 + slot * 40);
    }

    private static void ApplyRelationStyle(LinkModel link, string relation)
    {
        switch (relation)
        {
            case "contains":
                link.Color = "#8a96a8";
                link.SelectedColor = "#4d607a";
                link.Width = 1.35;
                break;
            case "calls":
                link.Color = "#c2410c";
                link.SelectedColor = "#9a3412";
                link.Width = 3;
                link.TargetMarker = LinkMarker.Arrow;
                break;
            case "references":
                link.Color = "#2563eb";
                link.SelectedColor = "#1d4ed8";
                link.Width = 2;
                link.TargetMarker = LinkMarker.Arrow;
                break;
            default:
                link.Color = "#667085";
                link.SelectedColor = "#374151";
                link.Width = 1.75;
                break;
        }
    }
}
