using Blazor.Diagrams.Core.Models;

namespace SemanticGraph.Visualizer.Client.Models;

public sealed class SemanticGraphLinkModel : LinkModel
{
    public SemanticGraphLinkModel(
        GraphEdgeDto edge,
        SemanticGraphNodeModel source,
        SemanticGraphNodeModel target)
        : base(edge.Id, source, target)
    {
        Edge = edge;
    }

    public GraphEdgeDto Edge { get; }
}
