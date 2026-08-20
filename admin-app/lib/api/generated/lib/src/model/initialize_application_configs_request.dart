//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'initialize_application_configs_request.g.dart';

/// InitializeApplicationConfigsRequest
///
/// Properties:
/// * [targetId]
/// * [templateId]
@BuiltValue()
abstract class InitializeApplicationConfigsRequest implements Built<InitializeApplicationConfigsRequest, InitializeApplicationConfigsRequestBuilder> {
  @BuiltValueField(wireName: r'target_id')
  String get targetId;

  @BuiltValueField(wireName: r'template_id')
  String? get templateId;

  InitializeApplicationConfigsRequest._();

  factory InitializeApplicationConfigsRequest([void updates(InitializeApplicationConfigsRequestBuilder b)]) = _$InitializeApplicationConfigsRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(InitializeApplicationConfigsRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<InitializeApplicationConfigsRequest> get serializer => _$InitializeApplicationConfigsRequestSerializer();
}

class _$InitializeApplicationConfigsRequestSerializer implements PrimitiveSerializer<InitializeApplicationConfigsRequest> {
  @override
  final Iterable<Type> types = const [InitializeApplicationConfigsRequest, _$InitializeApplicationConfigsRequest];

  @override
  final String wireName = r'InitializeApplicationConfigsRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    InitializeApplicationConfigsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'target_id';
    yield serializers.serialize(
      object.targetId,
      specifiedType: const FullType(String),
    );
    if (object.templateId != null) {
      yield r'template_id';
      yield serializers.serialize(
        object.templateId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    InitializeApplicationConfigsRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required InitializeApplicationConfigsRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'target_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.targetId = valueDes;
          break;
        case r'template_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.templateId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  InitializeApplicationConfigsRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = InitializeApplicationConfigsRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
